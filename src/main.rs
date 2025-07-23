extern crate reqwest;
extern crate tokio;
extern crate clap;

mod ui;
mod args;
mod http;
mod metrics;
mod shutdown;
mod charts;
mod logger;

use args::TesterArgs;
use tracing::info;
use std::error::Error;
use clap::Parser;
use tokio::sync::{broadcast, mpsc, watch};
use crate::{charts::plot_metrics, metrics::Metrics, ui::{setup_render_ui, UiData}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init_logging();

    let args = TesterArgs::parse();

    let (shutdown_tx, _) = broadcast::channel::<u16>(1);
    let (ui_tx, _) = watch::channel(UiData::default());
    let (metrics_tx, metrics_rx) = mpsc::unbounded_channel::<Metrics>();

    let shutdown_handle = shutdown::setup_shutdown_handler(&shutdown_tx);
    let render_ui_handle = setup_render_ui(
        &args,
        &shutdown_tx,
        &ui_tx
    );
    let (metrics_aggregator_handle, metrics_handle) = metrics::setup_metrics_collector(
        &args,
        &shutdown_tx,
        metrics_rx,
        &ui_tx
    );
    let request_sender_handle = http::setup_request_sender(
        &args,
        &shutdown_tx,
        &metrics_tx
    );

    if request_sender_handle.is_none() {
        return Ok(());
    }

    let (_, _, _, metrics_result, _) = tokio::join!(
        shutdown_handle,
        render_ui_handle,
        metrics_aggregator_handle,
        metrics_handle,
        request_sender_handle.unwrap()
    );

    let metrics = metrics_result.expect("Metrics collector failed");

    if !metrics.is_empty() {
        info!("📈 Plotting charts...");

        plot_metrics(&metrics, &args).await.expect("Failed to plot charts");

        info!("📈 Charts saved in {}", args.charts_path);
    }

    std::process::exit(0);
}
