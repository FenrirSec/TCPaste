use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::Rng;
use std::path::{Path, PathBuf};
use std::io::ErrorKind;
use std::env;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_TCP_PORT: u16 = 8888;
const DEFAULT_HTTP_PORT: u16 = 80;
const DEFAULT_HIDDEN_PATH: &str = "tcpaste";
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

fn handle_http_connection(mut stream: TcpStream, hidden_path: String) {
    let mut buffer = [0u8; 1024];
    match stream.read(&mut buffer) {
        Ok(_) => {
            let request = String::from_utf8_lossy(&buffer[..]);

            let mut lines = request.lines();
            if let Some(request_line) = lines.next() {
                let parts: Vec<&str> = request_line.split_whitespace().collect();
                if parts.len() == 3 && parts[0] == "GET" {
                    let path = parts[1].trim_start_matches('/');

                    // Check if the request starts with the hidden path
                    if !path.starts_with(&hidden_path) {
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nNot Found").unwrap();
                        return;
                    }

                    // Remove the hidden path prefix to get the actual file path
                    let actual_path = path.strip_prefix(&hidden_path)
                        .unwrap_or("")
                        .trim_start_matches('/');

                    // Prevent path traversal
                    let requested_path = Path::new(FILES_DIR).join(actual_path);
                    
                    if !requested_path.starts_with(FILES_DIR) || requested_path.display().to_string().contains("..") {
                        stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid path specified.").unwrap();
                        return;
                    }

                    if actual_path.is_empty() {
                        // List all files
                        match fs::read_dir(FILES_DIR) {
                            Ok(entries) => {
                                let mut file_list = String::new();
                                for entry in entries {
                                    if let Ok(entry) = entry {
                                        let file_name = entry.file_name();
                                        let display_name = file_name.to_string_lossy().to_string();
                                        let link = format!("<a href=\"/{}/{}\">{}</a><br>", hidden_path, display_name, display_name);
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
                        match requested_path.canonicalize() {
                            Ok(canonical_path) => {
                                let files_canonical = PathBuf::from(FILES_DIR).canonicalize().unwrap_or_default();
                                
                                // Ensure the canonical path is within FILES_DIR
                                if !canonical_path.starts_with(&files_canonical) {
                                    stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\nAccess Denied").unwrap();
                                    return;
                                }

                                if let Ok(mut file) = File::open(&canonical_path) {
                                    let mut contents = String::new();
                                    file.read_to_string(&mut contents).unwrap();
                                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", contents.len(), contents);
                                    stream.write_all(response.as_bytes()).unwrap();
                                } else {
                                    stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nFile not found").unwrap();
                                }
                            }
                            Err(_) => {
                                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nFile not found").unwrap();
                            }
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
    println!("Usage: program_name [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -h, --help                Show this help message");
    println!("  --host <HOST>             IP address to bind (default: {})", DEFAULT_HOST);
    println!("  --tcp-port <PORT>         Port for the TCP server (default: {})", DEFAULT_TCP_PORT);
    println!("  --http-port <PORT>        Port for the HTTP server (default: {})", DEFAULT_HTTP_PORT);
    println!("  --hidden-path <PATH>      Hidden path prefix for HTTP access (default: {})", DEFAULT_HIDDEN_PATH);
    println!();
    println!("Example:");
    println!("  program_name --host 0.0.0.0 --tcp-port 9000 --http-port 8080 --hidden-path mysecret");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if help flag is present
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return;
    }

    // Parse command line arguments
    let mut host = DEFAULT_HOST.to_string();
    let mut tcp_port = DEFAULT_TCP_PORT;
    let mut http_port = DEFAULT_HTTP_PORT;
    let mut hidden_path = DEFAULT_HIDDEN_PATH.to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --host requires a value");
                    return;
                }
            }
            "--tcp-port" => {
                if i + 1 < args.len() {
                    tcp_port = args[i + 1].parse().unwrap_or(DEFAULT_TCP_PORT);
                    i += 2;
                } else {
                    eprintln!("Error: --tcp-port requires a value");
                    return;
                }
            }
            "--http-port" => {
                if i + 1 < args.len() {
                    http_port = args[i + 1].parse().unwrap_or(DEFAULT_HTTP_PORT);
                    i += 2;
                } else {
                    eprintln!("Error: --http-port requires a value");
                    return;
                }
            }
            "--hidden-path" => {
                if i + 1 < args.len() {
                    hidden_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --hidden-path requires a value");
                    return;
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                return;
            }
        }
    }

    // Ensure the "files" directory exists
    let _ = std::fs::create_dir(FILES_DIR);

    let files_dir = Arc::new(Mutex::new(()));
    let tcp_files_dir = Arc::clone(&files_dir);

    // Start TCP server
    let tcp_listener = TcpListener::bind((host.as_str(), tcp_port)).unwrap();
    eprintln!("TCP Server listening on {}:{}", host, tcp_port);

    let tcp_host = host.clone();
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
    eprintln!("Files accessible at: http://{}:{}/{}/", host, http_port, hidden_path);

    for stream in http_listener.incoming() {
        let stream = stream.unwrap();
        let hidden_path_clone = hidden_path.clone();
        thread::spawn(move || {
            handle_http_connection(stream, hidden_path_clone);
        });
    }
}