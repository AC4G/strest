extern crate reqwest;
extern crate async_trait;

use std::time::Duration;

use reqwest::{Client, Proxy, Request};
use tokio::{sync::broadcast, time::{sleep, Instant}};

use crate::{args::{HttpMethod, TesterArgs}, metrics::Metrics};

pub fn setup_request_sender(
    args: &TesterArgs,
    shutdown_tx: &broadcast::Sender<u16>,
    metrics_tx: &broadcast::Sender<Metrics>,
) -> Option<tokio::task::JoinHandle<()>> {
    let shutdown_tx = shutdown_tx.clone();
    let metrics_tx = metrics_tx.clone();

    let mut client_builder = Client::builder()
        .timeout(std::time::Duration::from_secs(10));

    if let Some(ref proxy_url) = args.proxy_url {
        match Proxy::all(proxy_url) {
            Ok(proxy) => {
                client_builder = client_builder.proxy(proxy);
            }
            Err(e) => {
                eprintln!("Invalid proxy URL '{}': {}", proxy_url, e);
                let _ = shutdown_tx.send(1);
                return None;
            }
        }
    }

    let client = match client_builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to build HTTP client: {}", e);
            let _ = shutdown_tx.send(1);
            return None;
        }
    };

    let mut request_builder = match args.method {
        HttpMethod::Get => client.get(&args.url),
        HttpMethod::Post => client.post(&args.url),
        HttpMethod::Patch => client.patch(&args.url),
        HttpMethod::Put => client.put(&args.url),
        HttpMethod::Delete => client.delete(&args.url),
    };

    for (key, value) in &args.headers {
        request_builder = request_builder.header(key, value);
    }

    let request = match request_builder.body(args.data.clone()).build() {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to build request: {}", e);
            let _ = shutdown_tx.send(1);
            return None;
        }
    };

    Some(create_sender_task(
        shutdown_tx,
        metrics_tx,
        client,
        request,
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
            let test_request = request.try_clone().unwrap_or(request.try_clone().expect("Failed to clone request"));

            if let Err(e) = client.execute(test_request).await {
                eprintln!(" Test request failed: {}", e);
                return;
            }

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
