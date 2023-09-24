use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version, about = "Simple stress tester")]
pub struct TesterArgs {
    /// Sets the HTTP method
    #[arg(
        long,
        short = 'X',
        required = false,
        ignore_case = true,
        default_value = "get",
        value_parser = ["get", "post", "patch", "put", "delete"]
    )]
    pub method: String,
    /// Sets the URL to test
    #[arg(long, short, required = true)]
    pub url: String,
    /// Sets custom headers
    #[arg(long, short = 'H', required = false)]
    pub headers: Vec<String>,
    /// Sets the request data
    #[arg(long, short, required = false, default_value = "")]
    pub data: String,
    /// Sets the number of requests
    #[arg(long, short, default_value = "10000")]
    pub requests: u64
}
