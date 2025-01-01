use crate::errors::{Error, Result};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Server { addr }
    }

    fn send_response(stream: &mut TcpStream) -> Result<()> {
        let result = String::from("HTTP/1.1 200 OK\r\n\r\n");

        stream.write_all(result.as_bytes()).map_err(Error::Io)
    }

    pub fn listen(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)?;

        for stream in listener.incoming() {
            stream
                .map_err(|e| e.into())
                .and_then(|mut stream| Self::send_response(&mut stream))?
        }

        Ok(())
    }
}
