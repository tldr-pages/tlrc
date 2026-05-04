use std::borrow::Cow;
use std::fmt::Display;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use log::error;
use yansi::Paint;

use crate::util;

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

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub const DESC_NO_INTERNET: &'static str =
        "\n\nAn internet connection is required to download pages for the first time.";

    pub const DESC_AUTO_UPDATE_ERR: &'static str = "\n\nAn error occurred during the automatic update.\n\
        To skip updating the cache, run tldr with --offline.";

    pub const TRY_NO_EXPLICIT_LANGUAGE: &'static str = "Try running tldr without --language.";

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
        self.message = format!("{} {description}", self.message);
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

    pub fn desc_page_does_not_exist(cache_age: Duration) -> String {
        format!(
            "{}\
            If the page does not exist, you can create an issue or\n\
            document it yourself and create a pull request here:\n{}",
            if cache_age > Duration::from_secs(util::DAY) {
                Cow::Owned(format!(
                    "Last update was {} ago.\nIf that's not a typo, you could try running 'tldr --update'.\n\n",
                    util::duration_fmt(cache_age.as_secs()).bold()
                ))
            } else {
                // If the cache has been updated in the last 24 hours, don't suggest running 'tldr --update'.
                Cow::Borrowed("\n\n")
            },
            "https://github.com/tldr-pages/tldr".bold()
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
        error!("{}", self.message);

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
