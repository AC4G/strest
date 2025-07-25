use clap::{Parser, ValueEnum};

use crate::metrics::MetricsRange;

#[derive(Debug, Clone, ValueEnum)]
pub enum HttpMethod {
    Get,
    Post,
    Patch,
    Put,
    Delete,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LoadPattern {
    RampUp,
    Constant,
    Spike,
    Wave,
    Step,
}

#[derive(Debug, Parser, Clone)]
#[clap(version, about = "Simple HTTP stress tester")]
pub struct TesterArgs {
    /// HTTP method to use
    #[arg(
        long,
        short = 'X',
        default_value = "get",
        ignore_case = true
    )]
    pub method: HttpMethod,

    /// Target URL for the stress test
    #[arg(long, short)]
    pub url: String,

    /// HTTP headers in 'Key: Value' format (repeatable)
    #[arg(long, short = 'H', value_parser = parse_header)]
    pub headers: Vec<(String, String)>,

    /// Request body data (for POST/PUT)
    #[arg(long, short, default_value = "")]
    pub data: String,

    /// Duration of test (seconds)
    #[arg(long = "duration", short = 't', default_value = "30")]
    pub target_duration: u64,

    /// Expected HTTP status code
    #[arg(long = "status", short = 's', default_value = "200")]
    pub expected_status_code: u16,

    /// Path to save charts to
    #[arg(long, short = 'c', default_value = "./charts")]
    pub charts_path: String,

    /// Disable chart generation
    #[arg(long, short = 'n')]
    pub no_charts: bool,

    /// Proxy URL (optional)
    #[arg(long, short = 'p')]
    pub proxy_url: Option<String>,

    /// Max number of concurrent tasks for each request worker (default: 1000)
    #[arg(long, short = 'm', default_value = "1000")]
    pub max_tasks: usize,

    /// Number of tasks to spawn per tick (default: 1)
    #[arg(long = "spawn-rate", short = 'r', default_value = "1")]
    pub spawn_rate_per_tick: usize,

    /// Interval between ticks (milliseconds) (default: 100)
    #[arg(long = "spawn-interval", short = 'i', default_value = "100")]
    pub tick_interval: u64,

    /// Range, in seconds, of metrics to collect for charts (e.g., 10-30)
    #[arg(long = "metrics-range", short = 'M', value_parser, required = false)]
    pub metrics_range: Option<MetricsRange>,
}

fn parse_header(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid header format: '{}'. Expected 'Key: Value'", s));
    }
    let key = parts[0].trim().to_string();
    let value = parts[1].trim().to_string();
    Ok((key, value))
}
