extern crate reqwest;
extern crate tokio;
extern crate clap;

mod ui;
mod args;
mod http;

use http::{HttpMethodRequest, HttpRequest};
use args::TesterArgs;
use reqwest::Client;
use std::{error::Error, sync::{Arc, Mutex, mpsc}, time::{Duration, Instant}};
use clap::Parser;
use serde_json;
use ansi_term::Colour;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, Event, read};
use ui::{UiData, Ui, UiActions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = TesterArgs::parse();

    let url: String = args.url;
    let method: String = args.method.to_lowercase();
    let headers: Vec<String> = args.headers;
    let data: String = args.data;
    let num_requests: u64 = args.requests;

    let client: Client = reqwest::Client::new();
    let success_count = Arc::new(Mutex::new(0));
    let total_successful_time = Arc::new(Mutex::new(Duration::default()));
    let error_messages = Arc::new(Mutex::new(Vec::<String>::new()));

    let start_time = Instant::now();

    let (tx, rx) = mpsc::channel();

    let ui_thread = std::thread::spawn(move || {
        let mut terminal = Ui::setup_terminal().unwrap();

        let mut elapsed_time = Duration::default();
        let mut estimated_duration = Duration::default();

        let mut ui_channels: Vec<_> = (0..100)
            .map(|_| {
                let (tx, rx) = mpsc::channel();
                (Arc::new(Mutex::new(tx)), rx)
            })
            .collect();

        loop {
            while let Ok(data) = rx.try_recv() {
                match data {
                    UiData::ElapsedAndEstimatedTime(elapsed, estimated) => {
                        elapsed_time = elapsed;
                        estimated_duration = estimated;
                    }
                    UiData::Terminate => {
                        Ui::cleanup();
                        return;
                    }
                }

                Ui::render_ui(&mut terminal, &elapsed_time, &estimated_duration);
            }

            while let Some((ui_data_sender, ui_data_receiver)) = ui_channels.pop() {
                while let Ok(ui_data) = ui_data_receiver.try_recv() {
                    match ui_data {
                        UiData::ElapsedAndEstimatedTime(elapsed, estimated) => {
                            elapsed_time = elapsed;
                            estimated_duration = estimated;
                        }
                        UiData::Terminate => {
                            Ui::cleanup();
                            return;
                        }
                    }
                }
                drop(ui_data_sender);
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    });

    let success_count_clone_requests = Arc::clone(&success_count);
    let total_successful_time_clone_requests = Arc::clone(&total_successful_time);
    let method_clone = method.clone();
    let url_clone = url.clone();
    let headers_clone = headers.clone();
    let data_clone = data.clone();
    let tx_clone = tx.clone();
    let error_messages_clone = Arc::clone(&error_messages);

    let requests_thread = tokio::spawn(async move {
        let client_clone = client.clone();

        loop {
            if *success_count_clone_requests.lock().unwrap() >= num_requests {
                tx_clone.send(UiData::Terminate).expect("Failed to send Termination");
                break;
            }

            let request_start_time = Instant::now();
            let response = HttpMethodRequest
                .send_request(&client_clone, &method_clone, &url_clone, &headers_clone, &data_clone)
                .await;

            match response {
                Ok(_) => {
                    let request_elapsed_time = Instant::now() - request_start_time;
                    *success_count_clone_requests.lock().unwrap() += 1;
                    *total_successful_time_clone_requests.lock().unwrap() += request_elapsed_time;
                }
                Err(err) => {
                    let error_message = format!("Request failed: {}", err);
                    error_messages_clone.lock().unwrap().push(error_message);
                    tx_clone.send(UiData::Terminate).expect("Failed to send Termination");
                    break;
                }
            }

            let elapsed_time = Instant::now().checked_duration_since(start_time);
            if let Some(elapsed_time) = elapsed_time {
                let estimated_duration = if *success_count_clone_requests.lock().unwrap() > 0 {
                    elapsed_time
                        .div_f32(*success_count_clone_requests.lock().unwrap() as f32)
                        .mul_f32(num_requests as f32)
                } else {
                    Duration::default()
                };

                tx_clone.send(UiData::ElapsedAndEstimatedTime(elapsed_time, estimated_duration)).expect("Failed to send ElapsedAndEstimatedTime");
            }
            

            std::thread::sleep(Duration::from_millis(1));
        }
    });

    let input_thread = std::thread::spawn(move || {
        loop {
            if let Event::Key(event) = read().expect("Failed to read line") {
                match event {
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: _,
                        state: _,
                    } => {
                        tx.send(UiData::Terminate).expect("Failed to send Termination");
                        break;
                    },
                    _ => {}
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    });

    let _ = requests_thread.await;

    Ui::cleanup();
    clear_console();

    if !error_messages.lock().unwrap().is_empty() {
        println!("Error Message: {}\n", Colour::Red.paint(&error_messages.lock().unwrap()[0]));
    }

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

    println!("Total time taken: {:.2}s", Colour::Green.paint(format!("{:.2}", elapsed_time.as_secs_f64())));

    if elapsed_time.as_secs_f64() > 0.0 {
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

    ui_thread.join().expect("UI thread panicked");
    input_thread.join().expect("Input thread panicked");

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
