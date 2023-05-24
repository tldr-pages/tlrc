use std::fmt::Display;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::result::Result as StdResult;

use yansi::{Color, Paint};

pub enum ErrorKind {
    ParseToml,
    ParsePage,
    Download,
    Io,
    Msg,
}

pub struct Error {
    pub kind: ErrorKind,
    message: String,
}

pub type Result<T> = StdResult<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error {
    pub fn new<T>(message: T) -> Self
    where
        T: Display,
    {
        Self {
            kind: ErrorKind::Msg,
            message: message.to_string(),
        }
    }

    /// Set the `ErrorKind`.
    pub fn kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }

    /// Append `description` to the error message.
    pub fn describe<T>(mut self, description: T) -> Self
    where
        T: Display,
    {
        self.message = format!("{self} {description}");
        self
    }

    pub fn parse_page(page_path: &Path, i: usize, line: &str) -> Self {
        Error::new(format!(
            "'{}' is not a valid tldr page. (line {}):\n\n    {}",
            page_path.display(),
            i + 1,
            Paint::new(line).bold(),
        ))
        .kind(ErrorKind::ParsePage)
    }

    /// Print the error message to stderr and exit.
    pub fn exit(self) -> ! {
        writeln!(
            io::stderr(),
            "{} {self}",
            Paint::new("error:").fg(Color::Red).bold()
        )
        .unwrap_or_default();

        process::exit(match self.kind {
            ErrorKind::Msg | ErrorKind::Io => 1,
            ErrorKind::ParseToml => 3,
            ErrorKind::Download => 4,
            ErrorKind::ParsePage => 5,
        });
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::new(e).kind(ErrorKind::Io)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::new(e).kind(ErrorKind::ParseToml)
    }
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::new(e).kind(ErrorKind::Download)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::new(e).kind(ErrorKind::Download)
    }
}
