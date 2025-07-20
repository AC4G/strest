use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum HttpMethod {
    Get,
    Post,
    Patch,
    Put,
    Delete,
}

#[derive(Debug, Parser)]
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
    #[arg(long = "charts", short = 'c', default_value = "./charts")]
    pub charts_path: String,

    /// Proxy URL (optional)
    #[arg(long = "proxy", short = 'p')]
    pub proxy_url: Option<String>,
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
