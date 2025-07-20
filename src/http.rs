extern crate reqwest;
extern crate async_trait;

use std::time::Duration;

use reqwest::{Client, Request};
use tokio::{sync::broadcast, time::{sleep, Instant}};

use crate::{args::{HttpMethod, TesterArgs}, metrics::Metrics};

pub fn setup_request_sender(
    args: &TesterArgs,
    shutdown_tx: &broadcast::Sender<u16>,
    metrics_tx: &broadcast::Sender<Metrics>,
) -> Option<tokio::task::JoinHandle<()>> {
    let shutdown_tx = shutdown_tx.clone();
    let metrics_tx = metrics_tx.clone();

    let client: Client = Client::new();
    let method = &args.method;
    let url = &args.url;
    let data = &args.data;
    let headers = &args.headers;

    let mut request = match method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
        HttpMethod::Patch => client.patch(url),
        HttpMethod::Put => client.put(url),
        HttpMethod::Delete => client.delete(url)
    };

    for (key, value) in headers {
        request = request.header(key, value);
    }

    let body = reqwest::Body::from(data.clone());
    request = request.body(body);

    let built_request = match request.build() {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to build request: {e}");
            let _ = shutdown_tx.send(1);
            return None;
        }
    };

    Some(create_sender_task(
        shutdown_tx,
        metrics_tx,
        client.clone(),
        built_request
    ))
}

fn create_sender_task(
    shutdown_tx: broadcast::Sender<u16>,
    metrics_tx: broadcast::Sender<Metrics>,
    client: Client,
    request: Request
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            let mut concurrent = 0;

            loop {
                tokio::select! {
                    Ok(_) = shutdown_rx.recv() => break,

                    _ = interval.tick() => {
                        concurrent += 1;

                        for _ in 0..concurrent {
                            let shutdown_tx = shutdown_tx.clone();
                            let metrics_tx = metrics_tx.clone();
                            let client = client.clone();
                            let req = request.try_clone().unwrap();

                            tokio::spawn(async move {
                                let mut shutdown_rx = shutdown_tx.subscribe();

                                loop {
                                    tokio::select! {
                                        Ok(_) = shutdown_rx.recv() => break,
                                        _ = async {
                                            let start = Instant::now();
                                            let status = match client.execute(req.try_clone().unwrap()).await {
                                                Ok(resp) => resp.status().as_u16(),
                                                Err(_) => 500,
                                            };
                                            let _ = metrics_tx.send(Metrics::new(start, status));
                                        } => {}
                                    }

                                    sleep(Duration::from_millis(100)).await;
                                }
                            });
                        }
                    }
                }
            }
        })
    })
}
