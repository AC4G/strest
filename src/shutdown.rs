use std::time::Duration;

use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::broadcast;

pub fn setup_shutdown_handler(shutdown_tx: &broadcast::Sender<u16>) -> tokio::task::JoinHandle<()> {
    let shutdown_tx = shutdown_tx.clone();
    let mut shutdown_rx = shutdown_tx.subscribe();

    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::task::spawn_blocking(move || {
                loop {
                    if let Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) = read()
                    {
                        let _ = shutdown_tx.send(1);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }) => {},

            _ = shutdown_rx.recv() => return
        }
    })
}
