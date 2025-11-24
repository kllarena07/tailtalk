use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn handle_client(mut stream: TcpStream) {
    let addr = stream.peer_addr().unwrap();
    println!("Connected: {}", addr);
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf).unwrap();
        if n == 0 {
            println!("Disconnected: {}", addr);
            break;
        }

        std::io::stdout().write_all(&buf[..n]).unwrap();
        std::io::stdout().flush().unwrap();
    }
}

fn main() {
    let address = "0.0.0.0:2133";

    println!("Binding to port {}", address);

    let listener = TcpListener::bind(address).unwrap();

    for stream in listener.incoming() {
        handle_client(stream.unwrap());
    }
}
