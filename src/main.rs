use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let addr = stream.peer_addr()?;
    println!("Connected: {}", addr);
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            println!("Disconnected: {}", addr);
            break;
        }

        std::io::stdout().write_all(&buf[..n])?;
        std::io::stdout().flush()?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let address = "0.0.0.0:2133";

    println!("Binding to port {}", address);

    let listener = TcpListener::bind(address)?;

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                thread::spawn(|| {
                    if let Err(err) = handle_client(stream) {
                        eprintln!("Client handler error: {}", err);
                    }
                });
            }
            Err(err) => eprintln!("Accept error: {}", err),
        }
    }

    Ok(())
}
