use std::fmt::{self, Display};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::result::Result as StdResult;

use yansi::Paint;

#[derive(Debug)]
pub enum ErrorKind {
    ParseToml,
    ParsePage,
    Download,
    Io,
    Other,
}

#[derive(Debug)]
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
    pub const DESC_AUTO_UPDATE_ERR: &'static str =
        "\n\nAn error occurred during the automatic update. \
        To skip updating the cache, run tldr with --offline.";

    pub const DESC_LANG_NOT_INSTALLED: &'static str =
        "\n\nThe language you are trying to view the page in is not installed.\n\
        Please update your config and run 'tldr --update' to install a new language.";

    pub fn new<T>(message: T) -> Self
    where
        T: Display,
    {
        Self {
            kind: ErrorKind::Other,
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
            "'{}' is not a valid tldr page. (line {}):\n\n    {}\n",
            page_path.display(),
            i,
            line.bold(),
        ))
        .kind(ErrorKind::ParsePage)
    }

    pub fn parse_sumfile() -> Self {
        Error::new("could not parse the checksum file").kind(ErrorKind::Download)
    }

    pub fn desc_page_does_not_exist(try_update: bool) -> String {
        let e = if try_update {
            "Try running 'tldr --update'.\n\n"
        } else {
            "\n\n"
        };
        format!(
            "{e}\
            If the page does not exist, you can create an issue here:\n\
            {}\n\
            or document it yourself and create a pull request here:\n\
            {}",
            "https://github.com/tldr-pages/tldr/issues".bold(),
            "https://github.com/tldr-pages/tldr/pulls".bold()
        )
    }

    pub fn offline_no_cache() -> Self {
        Error::new("cache does not exist. Run tldr without --offline to download pages.")
            .kind(ErrorKind::Download)
    }

    pub fn messed_up_cache(e: &str) -> Self {
        Error::new(format!(
            "{e}\n\nThis should never happen, did you delete something from the cache?\n\
            Please run 'tldr --clean-cache' followed by 'tldr --update' to redownload all pages."
        ))
    }

    /// Print the error message to stderr and return an appropriate `ExitCode`.
    pub fn exit_code(self) -> ExitCode {
        let _ = writeln!(io::stderr(), "{} {self}", "error:".red().bold());

        match self.kind {
            ErrorKind::Other | ErrorKind::Io => 1,
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
