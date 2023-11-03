use std::fmt::{self, Display};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl Error {
    pub const DESC_DOWNLOAD_ERR: &str = "\n\nA download error occurred. \
        To skip updating the cache, run tldr with --offline.";

    pub const DESC_LANG_NOT_INSTALLED: &str =
        "\n\nThe language you are trying to view the page in \
        may not be installed (it's not defined in the config).\n\
        You can run 'tldr --info' to see currently installed languages.\n\
        Please update your config and run 'tldr --update' to install a new language.";

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
            i,
            Paint::new(line).bold(),
        ))
        .kind(ErrorKind::ParsePage)
    }

    pub fn parse_sumfile() -> Self {
        Error::new("failed to parse the checksum file").kind(ErrorKind::Download)
    }

    pub fn desc_page_does_not_exist() -> String {
        format!(
            "Try running 'tldr --update'.\n\n\
            If the page does not exist, you can create an issue here:\n\
            {}\n\
            or document it yourself and create a pull request here:\n\
            {}",
            Paint::new("https://github.com/tldr-pages/tldr/issues").bold(),
            Paint::new("https://github/com/tldr-pages/tldr/pulls").bold()
        )
    }

    /// Print the error message to stderr and return an appropriate `ExitCode`.
    pub fn exit_code(self) -> ExitCode {
        let _ = writeln!(
            io::stderr(),
            "{} {self}",
            Paint::new("error:").fg(Color::Red).bold()
        );

        match self.kind {
            ErrorKind::Msg | ErrorKind::Io => 1,
            ErrorKind::ParseToml => 3,
            ErrorKind::Download => 4,
            ErrorKind::ParsePage => 5,
        }
        .into()
    }
}

macro_rules! from_impl {
    ( $from:ty, $kind:tt ) => {
        impl From<$from> for Error {
            fn from(e: $from) -> Self {
                Error::new(e).kind(ErrorKind::$kind)
            }
        }
    };
}

from_impl! { io::Error, Io }
from_impl! { toml::de::Error, ParseToml }
from_impl! { ureq::Error, Download }
from_impl! { zip::result::ZipError, Download }
