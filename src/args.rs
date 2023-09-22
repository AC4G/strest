use clap::Parser;

///Simple URL Tester
#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct TesterArgs {
    ///Sets the HTTP method
    #[arg(
        long,
        short = 'X', 
        required = true,
        ignore_case = true,
        default_value = "get",
        value_parser = [
            "get", 
            "post", 
            "patch", 
            "put", 
            "delete"
        ])
    ]
    pub method: String,
    ///Sets the URL to test
    #[arg(long, short, required = true)]
    pub url: String,
    ///Sets custom headers
    #[arg(long, short = 'H', required = false)]
    pub headers: Vec<String>,
    ///Sets the request data
    #[arg(long, short, required = false)]
    pub data: String,
    ///Sets the number of requests
    #[arg(long, short, default_value = "10000")]
    pub requests: u64,
    ///Sets the number of parallel tasks
    #[arg(long, short, default_value = "4")]
    pub tasks: usize
}


