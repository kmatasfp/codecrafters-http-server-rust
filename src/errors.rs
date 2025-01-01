use derive_more::From;
use std::string::FromUtf8Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
#[allow(dead_code)]
pub enum Error {
    InvalidRequest,

    #[from]
    InvalidEncoding(FromUtf8Error),

    InvalidProtocol,
    InvalidMethod,

    #[from]
    Io(std::io::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
