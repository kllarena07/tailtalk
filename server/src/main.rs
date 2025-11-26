use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
use serde_json;
use indexmap::IndexMap;

struct Client {
    stream: TcpStream,
    username: String,
}

fn broadcast_message(
    message: &[u8],
    sender_addr: SocketAddr,
    connections: &Arc<Mutex<IndexMap<SocketAddr, Client>>>,
    include_sender: bool,
) -> io::Result<()> {
    let mut conn_map = connections.lock().unwrap();
    let mut to_remove = Vec::new();

    for (addr, client) in conn_map.iter_mut() {
        if include_sender || *addr != sender_addr {
            match client.stream.write_all(message) {
                Ok(_) => match client.stream.flush() {
                    Ok(_) => {}
                    Err(_) => to_remove.push(*addr),
                },
                Err(_) => to_remove.push(*addr),
            }
        }
    }

    for addr in to_remove {
        conn_map.shift_remove(&addr);
        println!(
            "Removed dead connection: {} (Total: {})",
            addr,
            conn_map.len()
        );
    }

    Ok(())
}

fn broadcast_user_list(connections: &Arc<Mutex<IndexMap<SocketAddr, Client>>>) -> io::Result<()> {
    let conn_map = connections.lock().unwrap();
    let user_list: Vec<String> = conn_map.values().map(|client| client.username.clone()).collect();
    drop(conn_map);

    let user_list_json = format!("USER_LIST:{}\n", serde_json::to_string(&user_list).unwrap());
    broadcast_message(user_list_json.as_bytes(), "0.0.0.0:0".parse().unwrap(), connections, true)
}

fn handle_user_list_request(
    mut stream: TcpStream,
    connections: &Arc<Mutex<IndexMap<SocketAddr, Client>>>,
) -> io::Result<()> {
    let conn_map = connections.lock().unwrap();
    let user_list: Vec<String> = conn_map.values().map(|client| client.username.clone()).collect();
    drop(conn_map);

    let user_list_json = format!("USER_LIST:{}\n", serde_json::to_string(&user_list).unwrap());
    stream.write_all(user_list_json.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn get_username(mut stream: &TcpStream, connections: &Arc<Mutex<IndexMap<SocketAddr, Client>>>) -> io::Result<String> {
    loop {
        stream.write_all(b"Enter your username: ")?;
        stream.flush()?;

        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf)?;

        let username = String::from_utf8_lossy(&buf[..n]).trim().to_string();

        if username.is_empty() {
            stream.write_all(b"Username cannot be empty. Please try again.\n")?;
            stream.flush()?;
            continue;
        }

        if username.eq_ignore_ascii_case("System") {
            stream.write_all(b"Username 'System' is reserved. Please choose another.\n")?;
            stream.flush()?;
            continue;
        }

        let conn_map = connections.lock().unwrap();
        let username_taken = conn_map.values().any(|client| client.username.eq_ignore_ascii_case(&username));
        drop(conn_map);

        if username_taken {
            stream.write_all(b"Username is already taken. Please choose another.\n")?;
            stream.flush()?;
            continue;
        }

        return Ok(username);
    }
}

fn handle_client(
    mut stream: TcpStream,
    connections: Arc<Mutex<IndexMap<SocketAddr, Client>>>,
) -> io::Result<()> {
    let addr = stream.peer_addr()?;

    let username = get_username(&stream, &connections)?;

    let mut conn_map = connections.lock().unwrap();
    conn_map.insert(
        addr,
        Client {
            stream: stream.try_clone()?,
            username: username.clone(),
        },
    );
    let total = conn_map.len();
    drop(conn_map);
    println!("{} connected from {} (Total: {})", username, addr, total);

    let join_message = format!("{} has joined the chat\n", username);
    broadcast_message(join_message.as_bytes(), addr, &connections, false)?; // Don't send to sender
    
    // Broadcast updated user list to all clients (including the new one)
    broadcast_user_list(&connections)?;

    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }

        let message = String::from_utf8_lossy(&buf[..n]);
        
        // Check for special commands
        if message.trim() == "GET_USERS" {
            handle_user_list_request(stream.try_clone()?, &connections)?;
            continue;
        }
        
        let formatted_msg = format!("{}: {}", username, message);

        std::io::stdout().write_all(formatted_msg.as_bytes())?;
        std::io::stdout().flush()?;

        broadcast_message(formatted_msg.as_bytes(), addr, &connections, false)?;
    }

    let leave_message = format!("{} has left the chat\n", username);
    broadcast_message(leave_message.as_bytes(), addr, &connections, false)?;
    
    // Remove client first, then broadcast updated user list
    let mut conn_map = connections.lock().unwrap();
    conn_map.shift_remove(&addr);
    let total = conn_map.len();
    drop(conn_map);
    println!("{} disconnected from {} (Total: {})", username, addr, total);
    
    broadcast_user_list(&connections)?;

    Ok(())
}

fn main() -> io::Result<()> {
    let address = "0.0.0.0:2133";

    println!("Binding to port {}", address);

    let listener = TcpListener::bind(address)?;
    let connections: Arc<Mutex<IndexMap<SocketAddr, Client>>> = Arc::new(Mutex::new(IndexMap::new()));

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let connections_clone = Arc::clone(&connections);
                thread::spawn(move || {
                    if let Err(err) = handle_client(stream, connections_clone) {
                        eprintln!("Client handler error: {}", err);
                    }
                });
            }
            Err(err) => eprintln!("Accept error: {}", err),
        }
    }

    Ok(())
}
