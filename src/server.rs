use crate::errors::{Error, Result};
use crate::thread_pool::ThreadPool;
use crate::Args;
use std::fs;
use std::sync::Arc;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
pub struct HttpRequest {
    target: String,
    headers: HashMap<String, String>,
}

impl TryFrom<&TcpStream> for HttpRequest {
    type Error = Error;

    fn try_from(stream: &TcpStream) -> Result<Self> {
        let buf_reader = BufReader::new(stream);

        let mut lines = buf_reader.lines();

        let maybe_request_line = if let Some(line) = lines.next() {
            let request_line = line?;

            let request_line_split: Vec<&str> = request_line.split_whitespace().collect();

            request_line_split
                .get(1)
                .ok_or(Error::InvalidRequest)
                .map(|t| (*t).to_owned())
        } else {
            Err(Error::InvalidRequest)
        };

        let mut headers: HashMap<String, String> = HashMap::new();
        for line in lines {
            let header_line = line?;

            if header_line.trim().is_empty() {
                break;
            }

            if let Some((key, value)) = header_line.split_once(':') {
                headers.insert(
                    key.trim().to_lowercase().to_owned(),
                    value.trim().to_owned(),
                );
            } else {
                return Err(Error::InvalidRequest);
            }
        }

        maybe_request_line.map(|target| HttpRequest { target, headers })
    }
}

pub struct Server {
    addr: String,
    conf: Args,
}

impl Server {
    pub fn new(addr: String, conf: Args) -> Self {
        Server { addr, conf }
    }

    fn handle_request(req: &HttpRequest, stream: &mut TcpStream, conf: &Args) -> Result<()> {
        let result = if req.target == "/" {
            String::from("HTTP/1.1 200 OK\r\n\r\n")
        } else if req.target.starts_with("/file") {
            if let Some(parent_dir) = &conf.directory {
                if let Some(file_name) = req.target.split('/').last() {
                    let file_path = parent_dir.join(file_name);
                    if file_path.exists() {
                        if let Ok(contents) = fs::read_to_string(file_path) {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                contents.len(),
                                contents
                            )
                        } else {
                            String::from("HTTP/1.1 500 Internal Server Error\r\n\r\n")
                        }
                    } else {
                        String::from("HTTP/1.1 404 Not Found\r\n\r\n")
                    }
                } else {
                    String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
                }
            } else {
                String::from("HTTP/1.1 503 Service Unavailable\r\n\r\n")
            }
        } else if req.target.starts_with("/echo") {
            if let Some(echo_str) = req.target.split('/').last() {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    echo_str.len(),
                    echo_str
                )
            } else {
                String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
            }
        } else if req.target.starts_with("/user-agent") {
            if let Some(user_agent_header) = req.headers.get("user-agent") {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    user_agent_header.len(),
                    user_agent_header
                )
            } else {
                String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
            }
        } else {
            String::from("HTTP/1.1 404 Not Found\r\n\r\n")
        };

        stream.write_all(result.as_bytes()).map_err(Error::Io)
    }

    pub fn listen(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)?;
        let pool = ThreadPool::new(8);

        let conf = Arc::new(self.conf.clone());

        for stream in listener.incoming() {
            let conf = Arc::clone(&conf);
            pool.execute(move || {
                match stream.map_err(|e| e.into()).and_then(|mut stream| {
                    HttpRequest::try_from(&stream)
                        .and_then(|req| Self::handle_request(&req, &mut stream, &conf))
                }) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to handle request, error {}", e),
                }
            });
        }

        Ok(())
    }
}
