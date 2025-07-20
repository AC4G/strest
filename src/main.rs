extern crate reqwest;
extern crate tokio;
extern crate clap;

mod ui;
mod args;
mod http;
mod metrics;
mod shutdown;
mod charts;

use args::TesterArgs;
use std::error::Error;
use clap::Parser;
use tokio::sync::broadcast;
use crate::{charts::plot_metrics, metrics::Metrics, ui::{setup_render_ui, UiData}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = TesterArgs::parse();

    // calculates the size of the metrics buffer based on the target duration
    let d = args.target_duration;
    let base_requests = 10 * d * (d + 1) / 2;
    let metrics_buffer_size = (base_requests as f64 * 1.2).ceil() as usize;

    let (shutdown_tx, _) = broadcast::channel::<u16>(1);
    let (ui_tx, _) = broadcast::channel::<UiData>(100);
    let (metrics_tx, metrics_rx) = broadcast::channel::<Metrics>(metrics_buffer_size);

    let shutdown_handle = shutdown::setup_shutdown_handler(&shutdown_tx);
    let render_ui_handle = setup_render_ui(
        &args,
        &shutdown_tx,
        &ui_tx
    );
    let metrics_handle = metrics::setup_metrics_collector(
        &args,
        &shutdown_tx,
        metrics_rx,
        &ui_tx,
        &metrics_buffer_size
    );
    let request_sender_handle = http::setup_request_sender(
        &args,
        &shutdown_tx,
        &metrics_tx
    );

    if request_sender_handle.is_none() {
        return Ok(());
    }

    let (_, _, metrics_result, _) = tokio::join!(
        shutdown_handle,
        render_ui_handle,
        metrics_handle,
        request_sender_handle.unwrap()
    );

    if !args.no_charts {
        let metrics = metrics_result.expect("Metrics collector failed");

        println!("📈 Plotting charts...");

        plot_metrics(&metrics, &args).await.expect("Failed to plot charts");

        println!("📈 Charts saved in {}", args.charts_path);
    }

    std::process::exit(0);
}
