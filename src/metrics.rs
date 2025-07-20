use std::{collections::VecDeque, time::Duration};

use tokio::{sync::{broadcast, mpsc}, task::JoinHandle, time::Instant};

use crate::{args::TesterArgs, ui::UiData};

#[derive(Clone, Debug)]
pub struct Metrics {
    pub start: Instant,
    pub response_time: Duration,
    pub status_code: u16
}

impl Metrics {
    pub fn new(start: Instant, status_code: u16) -> Self {
        Self {
            start,
            response_time: Instant::now() - start,
            status_code
        }
    }
}

pub fn setup_metrics_collector(
    args: &TesterArgs,
    shutdown_tx: &broadcast::Sender<u16>,
    mut resp_rx: broadcast::Receiver<Metrics>,
    ui_tx: &broadcast::Sender<UiData>,
    metrics_buffer_size: &usize
) -> JoinHandle<Vec<Metrics>> {
    let shutdown_tx_main = shutdown_tx.clone();
    let mut shutdown_rx = shutdown_tx_main.subscribe();
    let ui_tx = ui_tx.clone();

    let target_duration = Duration::from_secs(args.target_duration);
    let expected_status_code = args.expected_status_code;

    let (metrics_tx, mut metrics_rx) = mpsc::channel::<Metrics>(*metrics_buffer_size);

    let shutdown_tx_clone = shutdown_tx_main.clone();
    let ui_tx_clone = ui_tx.clone();

    tokio::spawn(async move {
        let mut latency_window: VecDeque<(Instant, f64)> = VecDeque::new();
        let mut request_timestamps: VecDeque<Instant> = VecDeque::new();
        let mut current_requests = 0;
        let mut successful_requests = 0;
        let start_time = Instant::now();
        let mut last_ui_update = Instant::now();
        let mut collected_metrics = Vec::new();

        let _ = ui_tx_clone.send(UiData::new(
            Duration::ZERO,
            0,
            0,
            vec![],
            0.0,
            0.0,
        ));

        loop {
            tokio::select! {
                Some(msg) = metrics_rx.recv() => {
                    let now = Instant::now();
                    let latency_ms = msg.response_time.as_secs_f64() * 1000.0;

                    collected_metrics.push(msg.clone());
                    current_requests += 1;

                    if msg.status_code == expected_status_code {
                        successful_requests += 1;
                    }

                    latency_window.push_back((now, latency_ms));
                    request_timestamps.push_back(now);

                    while let Some((ts, _)) = latency_window.front() {
                        if now.duration_since(*ts) > Duration::from_secs(10) {
                            latency_window.pop_front();
                        } else {
                            break;
                        }
                    }

                    while let Some(ts) = request_timestamps.front() {
                        if now.duration_since(*ts) > Duration::from_secs(60) {
                            request_timestamps.pop_front();
                        } else {
                            break;
                        }
                    }

                    let rps = request_timestamps
                        .iter()
                        .filter(|&&ts| now.duration_since(ts) <= Duration::from_secs(1))
                        .count() as f64;

                    let rpm = request_timestamps.len() as f64;

                    if last_ui_update.elapsed() >= Duration::from_millis(100) {
                        let elapsed_time = start_time.elapsed();
                        let recent_latencies: Vec<(f64, f64)> = latency_window
                            .iter()
                            .map(|&(ts, latency)| {
                                let secs_since_start = ts.duration_since(start_time).as_secs_f64();
                                (secs_since_start, latency)
                            })
                            .collect();
                        let _ = ui_tx_clone.send(UiData::new(
                            elapsed_time,
                            current_requests,
                            successful_requests,
                            recent_latencies,
                            rps,
                            rpm,
                        ));
                        last_ui_update = Instant::now();
                    }

                    if now.duration_since(start_time) >= target_duration {
                        let _ = shutdown_tx_clone.send(1);
                        break;
                    }
                },
                else => break,
            }
        }
    });

    tokio::spawn(async move {
        let mut metrics = Vec::new();

        loop {
            tokio::select! {
                Ok(_) = shutdown_rx.recv() => {
                    break;
                }
                Ok(msg) = resp_rx.recv() => {
                    let _ = metrics_tx.send(msg.clone()).await;
                    metrics.push(msg);
                }
            }
        }

        metrics
    })
}