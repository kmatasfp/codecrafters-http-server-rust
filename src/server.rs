use crate::errors::{Error, Result};
use crate::thread_pool::ThreadPool;
use crate::Args;
use std::fs;
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum HttpMethod {
    GET,
    DELETE,
    POST,
    PUT,
    HEAD,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

impl FromStr for HttpMethod {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "GET" => Ok(Self::GET),
            "DELETE" => Ok(Self::DELETE),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "HEAD" => Ok(Self::HEAD),
            "CONNECT" => Ok(Self::CONNECT),
            "OPTIONS" => Ok(Self::OPTIONS),
            "TRACE" => Ok(Self::TRACE),
            "PATCH" => Ok(Self::PATCH),
            _ => Err(Error::InvalidMethod),
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    target: String,
    method: HttpMethod,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl TryFrom<&TcpStream> for HttpRequest {
    type Error = Error;

    fn try_from(stream: &TcpStream) -> Result<Self> {
        let mut buf_reader = BufReader::new(stream);

        let mut lines = buf_reader.by_ref().lines();

        if let Some(line) = lines.next() {
            let request_line = line?;

            let request_line_split: Vec<&str> = request_line.split_whitespace().collect();

            let method = request_line_split
                .first()
                .ok_or(Error::InvalidRequest)
                .and_then(|method_str| HttpMethod::from_str(method_str))?;

            let request_target = request_line_split
                .get(1)
                .ok_or(Error::InvalidRequest)
                .map(|rt| (*rt).to_owned())?;

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

            let maybe_body = if let Some(content_length_str) = headers.get("content-length") {
                let content_length = content_length_str
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRequest)?;

                let mut buffer = vec![0; content_length];
                buf_reader.read_exact(&mut buffer)?;

                if !buffer.is_empty() {
                    Some(buffer)
                } else {
                    None
                }
            } else {
                None
            };

            Ok(HttpRequest {
                target: request_target,
                method,
                headers,
                body: maybe_body,
            })
        } else {
            Err(Error::InvalidRequest)
        }
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
        let response = match req {
            HttpRequest {
                target,
                method: HttpMethod::GET,
                headers: _,
                body: _,
            } if target == "/" => String::from("HTTP/1.1 200 OK\r\n\r\n"),
            HttpRequest {
                target,
                method: HttpMethod::POST,
                headers: _,
                body,
            } if target.starts_with("/file") => {
                if let Some(parent_dir) = &conf.directory {
                    if let Some(file_name) = target.split('/').last() {
                        let file_path = parent_dir.join(file_name);

                        if let Some(contents) = body {
                            if let Ok(()) = fs::write(file_path, contents) {
                                String::from("HTTP/1.1 201 Created\r\n\r\n")
                            } else {
                                String::from("HTTP/1.1 500 Internal Server Error\r\n\r\n")
                            }
                        } else {
                            String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
                        }
                    } else {
                        String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
                    }
                } else {
                    String::from("HTTP/1.1 503 Service Unavailable\r\n\r\n")
                }
            }
            HttpRequest {
                target,
                method: HttpMethod::GET,
                headers,
                body: _,
            } if target.starts_with("/file") => {
                if let Some(parent_dir) = &conf.directory {
                    if let Some(file_name) = target.split('/').last() {
                        let file_path = parent_dir.join(file_name);
                        if file_path.exists() {
                            if let Ok(contents) = fs::read_to_string(file_path) {
                                if let Some(encoding) = headers.get("accept-encoding") {
                                    if encoding.contains("gzip") {
                                        format!(
                                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{}",
                                            contents.len(),
                                            contents
                                        )
                                    } else {
                                        format!(
                                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                            contents.len(),
                                            contents
                                        )
                                    }
                                } else {
                                    format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                        contents.len(),
                                        contents
                                    )
                                }
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
            }
            HttpRequest {
                target,
                method: HttpMethod::GET,
                headers,
                body: _,
            } if target.starts_with("/echo") => {
                if let Some(echo_str) = target.split('/').last() {
                    if let Some(encoding) = headers.get("accept-encoding") {
                        if encoding.contains("gzip") {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/text-plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{}",
                                echo_str.len(),
                                echo_str
                            )
                        } else {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                echo_str.len(),
                                echo_str
                            )
                        }
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            echo_str.len(),
                            echo_str
                        )
                    }
                } else {
                    String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
                }
            }
            HttpRequest {
                target,
                method: HttpMethod::GET,
                headers,
                body: _,
            } if target.starts_with("/user-agent") => {
                if let Some(user_agent_header) = headers.get("user-agent") {
                    if let Some(encoding) = headers.get("accept-encoding") {
                        if encoding.contains("gzip") {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{}",
                                user_agent_header.len(),
                                user_agent_header
                            )
                        } else {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                user_agent_header.len(),
                                user_agent_header
                            )
                        }
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            user_agent_header.len(),
                            user_agent_header
                        )
                    }
                } else {
                    String::from("HTTP/1.1 400 Bad Request\r\n\r\n")
                }
            }
            _ => String::from("HTTP/1.1 404 Not Found\r\n\r\n"),
        };

        stream.write_all(response.as_bytes()).map_err(Error::Io)
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
