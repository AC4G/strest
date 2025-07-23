use std::{collections::VecDeque, time::Duration};

use tokio::{sync::{broadcast, mpsc, watch}, task::JoinHandle, time::Instant};

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
    mut metrics_collector_rx: mpsc::UnboundedReceiver<Metrics>,
    ui_tx: &watch::Sender<UiData>
) -> (JoinHandle<()>, JoinHandle<Vec<Metrics>>) {
    let shutdown_tx_main = shutdown_tx.clone();
    let mut shutdown_rx = shutdown_tx_main.subscribe();
    let ui_tx = ui_tx.clone();

    let target_duration = Duration::from_secs(args.target_duration);
    let expected_status_code = args.expected_status_code;
    let no_charts = args.no_charts;

    let (metrics_tx, mut metrics_rx) = mpsc::channel::<Metrics>(10_000);

    let forwarder_handle = tokio::spawn({
        async move {
            loop {
                tokio::select! {
                    Some(msg) = metrics_collector_rx.recv() => {
                        let _ = metrics_tx.try_send(msg);
                    },
                    _ = shutdown_rx.recv() => break,
                }
            }
        }
    });

    let metrics_aggregator_handle = tokio::spawn(async move {
        let mut latency_window: VecDeque<(Instant, f64)> = VecDeque::new();
        let mut rps_window: VecDeque<(Instant, usize)> = VecDeque::new();
        let mut current_requests = 0;
        let mut successful_requests = 0;
        let mut collected_metrics = Vec::new();
        let start_time = Instant::now();
        let mut last_ui_update = Instant::now();
        let mut shutdown_rx = shutdown_tx_main.subscribe();
        let ui_tx_clone = ui_tx.clone();

        let _ = ui_tx.send(UiData::new(
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

                    if !no_charts {
                        collected_metrics.push(msg.clone());
                    }

                    current_requests += 1;

                    if msg.status_code == expected_status_code {
                        successful_requests += 1;
                    }

                    latency_window.push_back((now, latency_ms));
                    while latency_window.front().map_or(false, |(ts, _)| now.duration_since(*ts) > Duration::from_secs(10)) {
                        latency_window.pop_front();
                    }

                    if let Some((ts, count)) = rps_window.back_mut() {
                        if now.duration_since(*ts) < Duration::from_millis(100) {
                            *count += 1;
                        } else {
                            rps_window.push_back((now, 1));
                        }
                    } else {
                        rps_window.push_back((now, 1));
                    }

                    while rps_window.front().map_or(false, |(ts, _)| now.duration_since(*ts) > Duration::from_secs(60)) {
                        rps_window.pop_front();
                    }

                    let rps: f64 = rps_window
                        .iter()
                        .filter(|(ts, _)| now.duration_since(*ts) <= Duration::from_secs(1))
                        .map(|(_, count)| *count)
                        .sum::<usize>() as f64;

                    let rpm = rps * 60.0;

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

                        last_ui_update = now;
                    }

                    if now.duration_since(start_time) >= target_duration {
                        let _ = shutdown_tx_main.send(1);
                        break;
                    }
                },
                _ = shutdown_rx.recv() => break,
            }
        }

        collected_metrics
    });

    (forwarder_handle, metrics_aggregator_handle)
}
