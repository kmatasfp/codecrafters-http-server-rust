use crate::errors::{Error, Result};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
pub struct HttpRequest {
    target: String,
}

impl TryFrom<&TcpStream> for HttpRequest {
    type Error = Error;

    fn try_from(stream: &TcpStream) -> Result<Self> {
        let buf_reader = BufReader::new(stream);

        let mut lines = buf_reader.lines();

        if let Some(line) = lines.next() {
            let request_line = line?;

            let request_line_split: Vec<&str> = request_line.split_whitespace().collect();

            let target = request_line_split
                .get(1)
                .ok_or(Error::InvalidRequest)
                .map(|t| (*t).to_owned())?;

            Ok(HttpRequest { target })
        } else {
            Err(Error::InvalidRequest)
        }
    }
}

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Server { addr }
    }

    fn handle_request(req: &HttpRequest, stream: &mut TcpStream) -> Result<()> {
        let result = if req.target == "/" {
            String::from("HTTP/1.1 200 OK\r\n\r\n")
        } else {
            String::from("HTTP/1.1 404 Not Found\r\n\r\n")
        };

        stream.write_all(result.as_bytes()).map_err(Error::Io)
    }

    pub fn listen(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)?;

        for stream in listener.incoming() {
            stream.map_err(|e| e.into()).and_then(|mut stream| {
                HttpRequest::try_from(&stream)
                    .and_then(|req| Self::handle_request(&req, &mut stream))
            })?
        }

        Ok(())
    }
}
