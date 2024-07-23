use std::net::{TcpListener, TcpStream};
use std::io::{Write, Read};
use std::sync::Mutex;
use rand::Rng;

const HOST: &str = "127.0.0.1";
const TCP_PORT: u16 = 8888;

fn generate_random_filename() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    format!("file_{}.txt", random_string)
}

fn handle_tcp_connection(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024];
    let filename = Mutex::new(generate_random_filename());

    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }

        let content = String::from_utf8_lossy(&buffer[..bytes_read]);

        let mut file_name = filename.lock().unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_name.as_str())
            .expect("Cannot open file");

        file.write_all(content.as_bytes()).unwrap();
        print!("{}", format!("Wrote file_{}.txt", file_name));
    }
}

fn main() {
    eprintln!("{}", format!("Server listening on {}:{}", HOST, TCP_PORT));
    let listener = TcpListener::bind((HOST, TCP_PORT)).unwrap();

    for stream in listener.incoming() {
        handle_tcp_connection(stream.unwrap());
    }
}
