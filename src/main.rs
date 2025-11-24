use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

fn broadcast_message(
    message: &[u8],
    sender_addr: SocketAddr,
    connections: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
) -> io::Result<()> {
    let mut conn_map = connections.lock().unwrap();
    let mut to_remove = Vec::new();

    for (addr, stream) in conn_map.iter_mut() {
        if *addr != sender_addr {
            match stream.write_all(message) {
                Ok(_) => match stream.flush() {
                    Ok(_) => {}
                    Err(_) => to_remove.push(*addr),
                },
                Err(_) => to_remove.push(*addr),
            }
        }
    }

    for addr in to_remove {
        conn_map.remove(&addr);
        println!(
            "Removed dead connection: {} (Total: {})",
            addr,
            conn_map.len()
        );
    }

    Ok(())
}

fn handle_client(
    mut stream: TcpStream,
    connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
) -> io::Result<()> {
    let addr = stream.peer_addr()?;

    let mut conn_map = connections.lock().unwrap();
    conn_map.insert(addr, stream.try_clone()?);
    let total = conn_map.len();
    drop(conn_map);
    println!("Added connection: {} (Total: {})", addr, total);

    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }

        std::io::stdout().write_all(&buf[..n])?;
        std::io::stdout().flush()?;

        broadcast_message(&buf[..n], addr, &connections)?;
    }

    let mut conn_map = connections.lock().unwrap();
    conn_map.remove(&addr);
    let total = conn_map.len();
    drop(conn_map);
    println!("Removed connection: {} (Total: {})", addr, total);

    Ok(())
}

fn main() -> io::Result<()> {
    let address = "0.0.0.0:2133";

    println!("Binding to port {}", address);

    let listener = TcpListener::bind(address)?;
    let connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> =
        Arc::new(Mutex::new(HashMap::new()));

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
