use std::{collections::BTreeMap, path::Path};

use plotters::prelude::*;
use tokio::fs;
use tracing::{error, info};

use crate::{args::TesterArgs, metrics::Metrics};

pub async fn plot_metrics(
    metrics: &Vec<Metrics>,
    args: &TesterArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = &args.charts_path;
    let expected_status_code = &args.expected_status_code;

    if let Err(e) = fs::create_dir_all(Path::new(path)).await {
        error!("Failed to create output directory '{}': {}", path, e);
        return Err(e.into());
    }
    
    info!("Plotting average response time...");

    plot_average_response_time(metrics, &format!("{}/average_response_time.png", path))
        .expect("Failed to plot average response time");

    info!("Plotting cumulative successful requests...");

    plot_cumulative_successful_requests(metrics, expected_status_code, &format!("{}/cumulative_successful_requests.png", path))
        .expect("Failed to plot successful requests");

    info!("Plotting cumulative error rate...");

    plot_cumulative_error_rate(metrics, expected_status_code, &format!("{}/cumulative_error_rate.png", path))
        .expect("Failed to plot error rate");

    info!("Plotting latency percentiles...");

    plot_latency_percentiles(metrics, &format!("{}/latency_percentiles", path))
        .expect("Failed to plot latency percentiles");

    info!("Plotting requests per second...");

    plot_requests_per_second(metrics, &format!("{}/requests_per_second.png", path))
        .expect("Failed to plot requests per second");

    info!("Plotting cumulative total requests...");

    plot_cumulative_total_requests(metrics, &format!("{}/cumulative_total_requests.png", path))
        .expect("Failed to plot cumulative total requests");

    Ok(())
}

fn plot_average_response_time(metrics: &Vec<Metrics>, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use plotters::prelude::*;

    let root = BitMapBackend::new(path, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let first_start = metrics[0].start;

    let mut buckets: BTreeMap<u64, Vec<u128>> = BTreeMap::new();

    for metric in metrics {
        let elapsed = metric.start.duration_since(first_start).as_secs_f64();
        let bucket_key = (elapsed * 10.0).floor() as u64; // 100ms granularity
        buckets.entry(bucket_key)
            .or_default()
            .push(metric.response_time.as_micros());
    }

    let mut data: Vec<(f64, u32)> = buckets.into_iter()
        .map(|(bucket, times)| {
            let avg_us = times.iter().sum::<u128>() as f64 / times.len() as f64;
            let avg_ms = avg_us / 1000.0;
            (bucket as f64 * 0.1, avg_ms as u32)
        })
        .collect();

    data.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let x_max = data.iter().map(|(x, _)| *x).fold(0.0, f64::max);
    let y_max = data.iter().map(|(_, y)| *y).max().unwrap_or(1000);

    let mut chart = ChartBuilder::on(&root)
        .caption("Average Response Time", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0.0..x_max, 0u32..y_max)?;

    chart.configure_mesh()
        .x_desc("Elapsed Time (seconds)")
        .y_desc("Avg Response Time (ms)")
        .x_labels(20)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(data.into_iter(), &BLUE))?;

    root.present()?;
    Ok(())
}

pub fn plot_cumulative_successful_requests(
    metrics: &Vec<Metrics>,
    expected_status_code: &u16,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use plotters::prelude::*;

    let root = BitMapBackend::new(path, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let first_start = metrics[0].start;
    let mut success_buckets: BTreeMap<u64, u32> = BTreeMap::new();

    for metric in metrics {
        if metric.status_code == *expected_status_code {
            let elapsed = metric.start.duration_since(first_start).as_secs_f64();
            let bucket = (elapsed * 10.0).floor() as u64; // 100ms buckets
            *success_buckets.entry(bucket).or_insert(0) += 1;
        }
    }

    let max_bucket = *success_buckets.keys().max().unwrap_or(&0);
    let mut cumulative = 0;
    let mut data: Vec<(f64, u32)> = Vec::with_capacity((max_bucket + 1) as usize);

    for bucket in 0..=max_bucket {
        let count = *success_buckets.get(&bucket).unwrap_or(&0);
        cumulative += count;
        data.push((bucket as f64 * 0.1, cumulative));
    }

    let x_max = data.last().map(|(x, _)| *x).unwrap_or(1.0);
    let y_max = data.last().map(|(_, y)| *y).unwrap_or(1);

    let mut chart = ChartBuilder::on(&root)
        .caption("Cumulative Successful Requests", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, 0u32..y_max)?;

    chart
        .configure_mesh()
        .x_desc("Elapsed Time (seconds)")
        .y_desc("Successful Requests")
        .x_labels(20)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(data.into_iter(), &BLUE))?;

    root.present()?;
    Ok(())
}

pub fn plot_cumulative_error_rate(
    metrics: &Vec<Metrics>,
    expected_status_code: &u16,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use plotters::prelude::*;

    let root = BitMapBackend::new(path, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let first_start = metrics[0].start;
    let mut error_buckets: BTreeMap<u64, u32> = BTreeMap::new();

    for metric in metrics {
        if metric.status_code != *expected_status_code {
            let elapsed = metric.start.duration_since(first_start).as_secs_f64();
            let bucket = (elapsed * 10.0).floor() as u64; // 100ms buckets
            *error_buckets.entry(bucket).or_insert(0) += 1;
        }
    }

    let max_bucket = *error_buckets.keys().max().unwrap_or(&0);
    let mut cumulative = 0;
    let mut data: Vec<(f64, u32)> = Vec::with_capacity((max_bucket + 1) as usize);

    for bucket in 0..=max_bucket {
        let count = *error_buckets.get(&bucket).unwrap_or(&0);
        cumulative += count;
        data.push((bucket as f64 * 0.1, cumulative));
    }

    let x_max = data.last().map(|(x, _)| *x).unwrap_or(1.0);
    let y_max = data.last().map(|(_, y)| *y).unwrap_or(1);

    let mut chart = ChartBuilder::on(&root)
        .caption("Cumulative Errors Over Time", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, 0u32..y_max)?;

    chart
        .configure_mesh()
        .x_desc("Elapsed Time (seconds)")
        .y_desc("Cumulative Errors")
        .x_labels(20)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(data.into_iter(), &RED))?;

    root.present()?;
    Ok(())
}

pub fn plot_latency_percentiles(
    metrics: &Vec<Metrics>,
    base_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    fn percentile(sorted: &[u128], pct: f64) -> u128 {
        let index = ((pct / 100.0) * sorted.len() as f64).round() as usize;
        *sorted.get(index.min(sorted.len() - 1)).unwrap_or(&0)
    }

    if metrics.is_empty() {
        return Ok(());
    }

    let first_start = metrics[0].start;

    let mut grouped: BTreeMap<u64, Vec<u128>> = BTreeMap::new();
    for m in metrics {
        let second = m.start.duration_since(first_start).as_secs();
        grouped.entry(second).or_default().push(m.response_time.as_micros());
    }

    let mut p50s = vec![];
    let mut p90s = vec![];
    let mut p99s = vec![];
    let mut seconds = vec![];

    for (sec, mut times) in grouped {
        times.sort_unstable();
        p50s.push(percentile(&times, 50.0) / 1000); // ms
        p90s.push(percentile(&times, 90.0) / 1000);
        p99s.push(percentile(&times, 99.0) / 1000);
        seconds.push(sec);
    }

    let y_max = *p99s.iter().max().unwrap_or(&100);

    fn draw_chart(
        seconds: &Vec<u64>,
        values: &Vec<u128>,
        title: &str,
        color: RGBColor,
        file_path: &str,
        y_max: u128,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = BitMapBackend::new(file_path, (1600, 600)).into_drawing_area();
        root.fill(&WHITE)?;

        let x_min = *seconds.first().unwrap_or(&0);
        let x_max = *seconds.last().unwrap_or(&0);

        let mut chart = ChartBuilder::on(&root)
            .caption(title, ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(30)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, 0u128..y_max)?;

        chart
            .configure_mesh()
            .x_desc("Elapsed Time (s)")
            .y_desc("Latency (ms)")
            .draw()?;

        let points: Vec<(u64, u128)> = seconds.iter().cloned().zip(values.iter().cloned()).collect();
        chart.draw_series(LineSeries::new(points, &color))?;

        root.present()?;
        Ok(())
    }

    draw_chart(
        &seconds,
        &p50s,
        "Latency P50",
        BLUE,
        &format!("{}_P50.png", base_path),
        y_max,
    )?;

    draw_chart(
        &seconds,
        &p90s,
        "Latency P90",
        GREEN,
        &format!("{}_P90.png", base_path),
        y_max,
    )?;

    draw_chart(
        &seconds,
        &p99s,
        "Latency P99",
        RED,
        &format!("{}_P99.png", base_path),
        y_max,
    )?;

    Ok(())
}

pub fn plot_requests_per_second(metrics: &Vec<Metrics>, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(path, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let first_start = metrics[0].start;

    let elapsed_secs: Vec<u64> = metrics.iter()
        .map(|m| m.start.duration_since(first_start).as_secs())
        .collect();

    let max_sec = *elapsed_secs.iter().max().unwrap();

    let mut counts = vec![0u32; (max_sec + 1) as usize];
    for &sec in &elapsed_secs {
        counts[sec as usize] += 1;
    }

    let x_range = 0u32..(max_sec as u32 + 1);
    let y_max = *counts.iter().max().unwrap_or(&1);
    let y_range = 0u32..(y_max + 1);

    let mut chart = ChartBuilder::on(&root)
        .caption("Requests per Second", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(x_range.clone(), y_range)?;

    chart.configure_mesh()
        .x_desc("Elapsed Time (seconds)")
        .y_desc("Requests per Second")
        .draw()?;

    chart.draw_series(LineSeries::new(
        counts.iter().enumerate().map(|(sec, &count)| (sec as u32, count)),
        &BLUE,
    ))?;

    root.present()?;
    Ok(())
}

pub fn plot_cumulative_total_requests(
    metrics: &Vec<Metrics>,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use plotters::prelude::*;

    let root = BitMapBackend::new(path, (1600, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let first_start = metrics[0].start;
    let mut total_buckets: BTreeMap<u64, u32> = BTreeMap::new();

    for metric in metrics {
        let elapsed = metric.start.duration_since(first_start).as_secs_f64();
        let bucket = (elapsed * 10.0).floor() as u64; // 100ms buckets
        *total_buckets.entry(bucket).or_insert(0) += 1;
    }

    let max_bucket = *total_buckets.keys().max().unwrap_or(&0);
    let mut cumulative = 0;
    let mut data: Vec<(f64, u32)> = Vec::with_capacity((max_bucket + 1) as usize);

    for bucket in 0..=max_bucket {
        let count = *total_buckets.get(&bucket).unwrap_or(&0);
        cumulative += count;
        data.push((bucket as f64 * 0.1, cumulative));
    }

    let x_max = data.last().map(|(x, _)| *x).unwrap_or(1.0);
    let y_max = data.last().map(|(_, y)| *y).unwrap_or(1);

    let mut chart = ChartBuilder::on(&root)
        .caption("Cumulative Total Requests", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, 0u32..y_max)?;

    chart
        .configure_mesh()
        .x_desc("Elapsed Time (seconds)")
        .y_desc("Cumulative Total Requests")
        .x_labels(20)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(data.into_iter(), &BLACK))?;

    root.present()?;
    Ok(())
}
