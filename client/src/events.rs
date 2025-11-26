use crate::app::Event;
use serde_json;
use std::{io::Read, net::TcpStream, sync::mpsc, thread, time::Duration};

pub fn handle_input_events(tx: mpsc::Sender<Event>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            crossterm::event::Event::Mouse(mouse_event) => {
                tx.send(Event::Mouse(mouse_event)).unwrap()
            }
            _ => {}
        }
    }
}

pub fn run_cursor_blink_thread(tx: mpsc::Sender<Event>) {
    let blink_duration = Duration::from_millis(500);
    loop {
        tx.send(Event::CursorBlink).unwrap();
        thread::sleep(blink_duration);
    }
}

pub fn handle_server_messages(mut stream: TcpStream, tx: mpsc::Sender<Event>) {
    let mut buf = [0u8; 4096];
    let mut buffer = String::new();

    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                // Server disconnected
                let _ = tx.send(Event::ServerMessage("Server disconnected".to_string()));
                break;
            }
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&buf[..n]);
                buffer.push_str(&chunk);

                // Process complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if !line.trim().is_empty() {
                        // Check if this is a user list update
                        if line.trim().starts_with("USER_LIST:") {
                            let trimmed_line = line.trim();
                            if let Some(json_part) = trimmed_line.strip_prefix("USER_LIST:") {
                                if let Ok(users) = serde_json::from_str::<Vec<String>>(json_part) {
                                    let _ = tx.send(Event::UserListUpdate(users));
                                }
                            }
                        } else {
                            let _ = tx.send(Event::ServerMessage(format!("{}\n", line)));
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ServerMessage(format!("Connection error: {}", e)));
                break;
            }
        }
    }
}
