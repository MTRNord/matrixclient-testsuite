use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("tried to parse invalid string")]
    StringFromBytesError {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error("tried to parse invalid identifier")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error("tried to parse invalid identifier")]
    RumaIdentifierError {
        #[from]
        source: ruma::identifiers::Error,
    },
    #[error("tried to parse invalid event")]
    RumaEventError {
        #[from]
        source: ruma::events::InvalidEvent,
    },
    #[error("bad request")]
    BadRequest(&'static str),
}

#[derive(Error, Debug)]
pub enum TestErrors {
    #[error(transparent)]
    Io(#[from] io::Error),
}
