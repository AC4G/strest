extern crate reqwest;
extern crate tokio;
extern crate clap;

mod args;
mod http;

use crate::http::HttpRequest;
use http::HttpMethodRequest;
use args::TesterArgs;
use reqwest::Client;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use clap::Parser;
use serde_json;
use ansi_term::Colour;

const REQUEST_INTERVAL_MS: f64 = 0.1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = TesterArgs::parse();

    let url: String = args.url;
    let method: String = args.method.to_lowercase();
    let headers: Vec<String> = args.headers;
    let data: String = args.data;
    let num_requests: u64 = args.requests;
    let num_tasks: usize = args.tasks;

    let client: Client = reqwest::Client::new();
    let success_count = Arc::new(Mutex::new(0));
    let total_successful_time = Arc::new(Mutex::new(Duration::default()));

    let mut tasks = vec![];
    let mut tasks_to_await = vec![];

    let start_time = Instant::now();

    for _ in 0..num_requests {
        let client_clone = client.clone();
        let success_count_clone = Arc::clone(&success_count);
        let total_successful_time_clone = Arc::clone(&total_successful_time);
        let url_clone = url.clone();
        let method_clone = method.clone();
        let headers_clone = headers.clone();
        let data_clone = data.clone();
    
        let task = tokio::spawn(async move {
            let request_start_time = Instant::now();
            let response = HttpMethodRequest
                .send_request(&client_clone, &method_clone, &url_clone, &headers_clone, &data_clone)
                .await;
    
            match response {
                Ok(_) => {
                    let request_elapsed_time = Instant::now() - request_start_time;
                    *success_count_clone.lock().unwrap() += 1;
                    *total_successful_time_clone.lock().unwrap() += request_elapsed_time;
                }
                Err(err) => {
                    eprintln!("Request failed: {:?}", err);
                }
            }
        });

        tasks.push(task);

        if tasks.len() >= num_tasks {
            tasks_to_await.append(&mut tasks.split_off(num_tasks));
        }

        if REQUEST_INTERVAL_MS > 0.0 {
            let delay_secs = REQUEST_INTERVAL_MS as u64;
            sleep(Duration::from_secs(delay_secs)).await;
        }

        let elapsed_time = Instant::now().checked_duration_since(start_time);
        if let Some(elapsed_time) = elapsed_time {
            let estimated_duration = if *success_count.lock().unwrap() > 0 {
                elapsed_time
                    .div_f32(*success_count.lock().unwrap() as f32)
                    .mul_f32(num_requests as f32)
            } else {
                Duration::default()
            };

            let message = format!(
                "\rTime elapsed: {:.2}s Estimated: {:.2}s",
                Colour::bold(Colour::Green).paint(format!("{:.2}", elapsed_time.as_secs_f64())),
                Colour::bold(Colour::Cyan).paint(format!("{:.2}", estimated_duration.as_secs_f64()))
            );

            print!("{}", message);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }

    tasks_to_await.extend(tasks);

    for task in tasks_to_await {
        task.await?;
    }

    println!("\r");

    clear_console();

    let success_count = *success_count.lock().unwrap();
    let total_successful_time = *total_successful_time.lock().unwrap();

    println!("Total parallel requests: {}", Colour::Green.paint(num_requests.to_string()));
    println!("Successful parallel requests: {}", Colour::Green.paint(success_count.to_string()));

    if success_count > 0 {
        let average_response_time =
            total_successful_time.as_secs_f64() * 1000.0 / success_count as f64;
        println!("Average response time: {:.2}ms", Colour::Green.paint(format!("{:.2}", average_response_time)));
    } else {
        println!("No successful requests made.");
    }

    let elapsed_time = Instant::now() - start_time;

    println!("Total time taken: {:.2}s", Colour::Green.paint(format!("{:.2}", elapsed_time.as_secs_f32())));

    if elapsed_time.as_secs() > 0 {
        let requests_per_minute =
            num_requests as f64 / elapsed_time.as_secs_f64() * 60.0;
        println!("Requests per minute: {:.2}", Colour::Green.paint(format!("{:.2}", requests_per_minute)));
    } else {
        println!("No requests made.");
    }

    println!("Against: {} {}", Colour::Green.paint(method.to_uppercase()), Colour::Green.paint(url));

    if !headers.is_empty() {
        let headers_str = headers
            .iter()
            .map(|header| format!("\t{}", header))
            .collect::<Vec<String>>()
            .join(",\n");
        println!("Headers: [\n{}\n]", Colour::Green.paint(headers_str));
    }

    if !data.is_empty() {
        if headers.iter().any(|header| header.to_lowercase().starts_with("content-type: application/json")) {
            let data_json = serde_json::to_string_pretty(&data)?;
            println!("Data: {}", Colour::Green.paint(data_json));
        } else {
            let data_str = data.split("&")
                .map(|data| format!("\t{}", data))
                .collect::<Vec<String>>()
                .join(",\n");
            println!("Data: [\n{}\n]", Colour::Green.paint(data_str));
        }
    }

    Ok(())
}

fn clear_console() {
    if cfg!(windows) {
        std::process::Command::new("cmd")
            .arg("/c")
            .arg("cls")
            .status()
            .expect("Failed to clear console");
    } else if cfg!(unix) {
        std::process::Command::new("clear")
            .status()
            .expect("Failed to clear console");
    }
}