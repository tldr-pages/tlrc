use std::io;
use std::process::exit;
use std::result::Result as StdResult;

use crate::util::error;


pub enum Error {
    ParseToml(String),
    HttpRequest(String),
    ExtractArchive(String),
    Argument(String),
    Msg(String),
}

pub type Result<T> = StdResult<T, Error>;

impl Error {
    /// Print the error message to stderr and exit.
    pub fn exit(self) -> ! {
        exit(match self {
            Error::Msg(desc) => {
                error(&desc);
                1
            },
            Error::Argument(desc) => {
                error(&desc);
                2
            },
            Error::ParseToml(desc) => {
                error(&desc);
                3
            },
            Error::HttpRequest(desc) | Error::ExtractArchive(desc) => {
                error(&desc);
                4
            },
        });
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Msg(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::ParseToml(e.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::HttpRequest(e.to_string())
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::ExtractArchive(e.to_string())
    }
}
