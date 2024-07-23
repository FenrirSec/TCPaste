use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::Rng;
use std::path::{Path};
use std::io::ErrorKind;

const HOST: &str = "127.0.0.1";
const TCP_PORT: u16 = 8888;
const HTTP_PORT: u16 = 80;
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
                eprintln!("Wrote to {}", file_path);
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

fn main() {
    // Ensure the "files" directory exists
    let _ = std::fs::create_dir(FILES_DIR);

    let files_dir = Arc::new(Mutex::new(()));

    let tcp_files_dir = Arc::clone(&files_dir);

    let tcp_listener = TcpListener::bind((HOST, TCP_PORT)).unwrap();
    eprintln!("TCP Server listening on {}:{}", HOST, TCP_PORT);

    thread::spawn(move || {
        for stream in tcp_listener.incoming() {
            let stream = stream.unwrap();
            let tcp_files_dir = Arc::clone(&tcp_files_dir);
            thread::spawn(move || {
                handle_tcp_connection(stream, tcp_files_dir);
            });
        }
    });

    let http_listener = TcpListener::bind((HOST, HTTP_PORT)).unwrap();
    eprintln!("HTTP Server listening on {}:{}", HOST, HTTP_PORT);

    for stream in http_listener.incoming() {
        let stream = stream.unwrap();
        thread::spawn(move || {
            handle_http_connection(stream);
        });
    }
}
