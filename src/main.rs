use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::Rng;
use std::path::{Path};
use std::io::ErrorKind;
use std::env;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_TCP_PORT: u16 = 8888;
const DEFAULT_HTTP_PORT: u16 = 80;
const TIMEOUT_DURATION: Duration = Duration::from_secs(10);
const FILES_DIR: &str = "files";

fn generate_random_filename() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    random_string
}

fn handle_tcp_connection(mut stream: TcpStream, files_dir: Arc<Mutex<()>>) {
    stream.set_read_timeout(Some(TIMEOUT_DURATION)).unwrap();
    let mut buffer = [0u8; 1024];
    let filename = generate_random_filename();
    let file_path = format!("{}/{}", FILES_DIR, filename);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .expect("Cannot open file");

    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    eprintln!("Client closed the connection, closing stream");
                    break;
                }

                let content = String::from_utf8_lossy(&buffer[..bytes_read]);

                let _lock = files_dir.lock().unwrap();
                file.write_all(content.as_bytes()).unwrap();
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                eprintln!("Connection timed out, closing stream");
                break;
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break;
            }
        }
    }
    eprintln!("Wrote to {}", file_path);
}

fn handle_http_connection(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024];
    match stream.read(&mut buffer) {
        Ok(_) => {
            let request = String::from_utf8_lossy(&buffer[..]);

            let mut lines = request.lines();
            if let Some(request_line) = lines.next() {
                let parts: Vec<&str> = request_line.split_whitespace().collect();
                if parts.len() == 3 && parts[0] == "GET" {
                    let path = parts[1].trim_start_matches('/');

                    // Prevent path traversal by ensuring the path stays within the FILES_DIR
                    let requested_path = Path::new(FILES_DIR).join(path);
                    
                    if !requested_path.starts_with(FILES_DIR) || requested_path.display().to_string().contains("..") {
                        stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid path specified.").unwrap();
                        return;
                    }

                    if path.is_empty() || path == "/" {
                        // List all files
                        match fs::read_dir(FILES_DIR) {
                            Ok(entries) => {
                                let mut file_list = String::new();
                                for entry in entries {
                                    if let Ok(entry) = entry {
                                        let file_name = entry.file_name();
                                        let display_name = file_name.to_string_lossy().to_string();
                                        let link = format!("<a href=\"/{}\">{}</a><br>", display_name, display_name);
                                        file_list.push_str(&link);
                                    }
                                }
                                let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}", file_list.len(), file_list);
                                stream.write_all(response.as_bytes()).unwrap();
                            }
                            Err(_) => {
                                stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\nUnable to read directory").unwrap();
                            }
                        }
                    } else {
                        // Serve specific file
                        let requested_path = requested_path.canonicalize().unwrap_or_default();
                        println!("Canonical path: {}", requested_path.display()); // TODO : Better check for Path Traversal

                        if let Ok(mut file) = File::open(&requested_path) {
                            let mut contents = String::new();
                            file.read_to_string(&mut contents).unwrap();
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", contents.len(), contents);
                            stream.write_all(response.as_bytes()).unwrap();
                        } else {
                            stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nFile not found").unwrap();
                        }
                    }
                } else {
                    stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid request").unwrap();
                }
            }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            eprintln!("HTTP connection timed out");
        }
        Err(e) => {
            eprintln!("Error reading HTTP stream: {}", e);
        }
    }
}

fn print_help() {
    println!("Usage: program_name [host] [tcp_port] [http_port]");
    println!();
    println!("Arguments:");
    println!("  -h, --help          Show this help message.");
    println!("  host                IP address to bind (default: 127.0.0.1).");
    println!("  tcp_port            Port for the TCP server (default: 8888).");
    println!("  http_port           Port for the HTTP server (default: 80).");
}

fn main() {
    // Retrieve host and ports from command line arguments
    let args: Vec<String> = env::args().collect();

    // Check if help flag is present
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return;
    }

    // Create a variable for the default host to ensure it has the correct lifetime
    let default_host = DEFAULT_HOST.to_string();
    let host = args.get(1).unwrap_or(&default_host);

    let tcp_port = args.get(2).map_or(DEFAULT_TCP_PORT, |port| port.parse().unwrap_or(DEFAULT_TCP_PORT));
    let http_port = args.get(3).map_or(DEFAULT_HTTP_PORT, |port| port.parse().unwrap_or(DEFAULT_HTTP_PORT));

    // Ensure the "files" directory exists
    let _ = std::fs::create_dir(FILES_DIR);

    let files_dir = Arc::new(Mutex::new(()));

    let tcp_files_dir = Arc::clone(&files_dir);

    // Start TCP server
    let tcp_listener = TcpListener::bind((host.as_str(), tcp_port)).unwrap();
    eprintln!("TCP Server listening on {}:{}", host, tcp_port);

    thread::spawn(move || {
        for stream in tcp_listener.incoming() {
            let stream = stream.unwrap();
            let tcp_files_dir = Arc::clone(&tcp_files_dir);
            thread::spawn(move || {
                handle_tcp_connection(stream, tcp_files_dir);
            });
        }
    });

    // Start HTTP server
    let http_listener = TcpListener::bind((host.as_str(), http_port)).unwrap();
    eprintln!("HTTP Server listening on {}:{}", host, http_port);

    for stream in http_listener.incoming() {
        let stream = stream.unwrap();
        thread::spawn(move || {
            handle_http_connection(stream);
        });
    }
}
