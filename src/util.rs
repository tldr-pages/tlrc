use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::io::{self, IsTerminal, Write};
use std::iter;
use std::mem;
use std::path::Path;

use clap::ColorChoice;
use log::debug;
use ring::digest::{digest, SHA256};

/// A simple logger for the `log` crate that logs to stderr.
pub struct Logger;

impl Logger {
    pub fn init(quiet: bool, verbose: u8) {
        let lvl = match (quiet, verbose) {
            (true, _) => log::LevelFilter::Error,  // --quiet
            (false, 0) => log::LevelFilter::Info,  // default log level
            (false, 1) => log::LevelFilter::Debug, // --verbose
            (false, _) => log::LevelFilter::Trace, // --verbose --verbose
        };

        // The logger isn't set anywhere else, this is safe to unwrap.
        log::set_logger(&Self).unwrap();
        log::set_max_level(lvl);
    }
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // This isn't needed, log::set_max_level is enough for such a simple use case.
        true
    }

    // stderr is flushed on every writeln! call.
    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        use yansi::Paint;

        let level = match record.level() {
            log::Level::Trace => "trace:".blue().bold(),
            log::Level::Debug => "debug:".magenta().bold(),
            log::Level::Info => "info:".cyan().bold(),
            log::Level::Warn => "warning:".yellow().bold(),
            log::Level::Error => "error:".red().bold(),
        };

        let _ = match record.target() {
            t if t.starts_with("tldr") => writeln!(io::stderr(), "{level} {}", record.args()),
            t => writeln!(io::stderr(), "{level} [{}] {}", t.bold(), record.args()),
        };
    }
}

/// Print a status message without a trailing newline.
/// If verbose logging is enabled, use `log::info!` normally.
macro_rules! info_start {
    ( $( $arg:tt )* ) => {
        if log::log_enabled!(log::Level::Debug) {
            log::info!($($arg)*);
        } else if log::log_enabled!(log::Level::Info) {
            use std::io::Write;
            use yansi::Paint;
            let mut stderr = std::io::stderr().lock();
            let _ = write!(stderr, "{} ", "info:".cyan().bold());
            let _ = write!(stderr, $($arg)*);
        }
    };
}

/// End the status message started using `info_start`.
/// If verbose logging is enabled, do nothing.
macro_rules! info_end {
    ( $( $arg:tt )* ) => {
        if !log::log_enabled!(log::Level::Debug) && log::log_enabled!(log::Level::Info) {
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), $($arg)*);
        }
    };
}

pub(crate) use {info_end, info_start};

/// Get languages from environment variables according to the tldr client specification.
pub fn get_languages_from_env(out_vec: &mut Vec<String>) {
    // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#language

    let Ok(var_lang) = env::var("LANG") else {
        debug!("LANG is not set, cannot get languages from env vars");
        return;
    };

    let var_language = env::var("LANGUAGE");

    let languages = var_language
        .as_deref()
        .unwrap_or_default()
        .split_terminator(':')
        .chain(iter::once(&*var_lang));

    for lang in languages {
        if lang.len() >= 5 && lang.chars().nth(2) == Some('_') {
            // <language>_<country> (ll_CC - 5 characters)
            out_vec.push(lang[..5].to_string());
            // <language> (ll - 2 characters)
            out_vec.push(lang[..2].to_string());
        } else if lang.len() == 2 {
            out_vec.push(lang.to_string());
        } else {
            debug!("invalid language found in LANG or LANGUAGE: '{lang}'");
        }
    }
}

/// Initialize color outputting.
pub fn init_color(color_mode: ColorChoice) {
    match color_mode {
        ColorChoice::Always => {}
        ColorChoice::Never => yansi::disable(),
        ColorChoice::Auto => {
            let no_color = env::var_os("NO_COLOR").is_some_and(|x| !x.is_empty());

            if no_color || !io::stdout().is_terminal() || !io::stderr().is_terminal() {
                yansi::disable();
            }
        }
    }
}

pub trait Dedup {
    /// Deduplicate a vector in place preserving the order of elements.
    fn dedup_nosort(&mut self);
}

impl<T> Dedup for Vec<T>
where
    T: PartialEq,
{
    fn dedup_nosort(&mut self) {
        let old = mem::replace(self, Vec::with_capacity(self.len()));
        for x in old {
            if !self.contains(&x) {
                self.push(x);
            }
        }
    }
}

pub trait PagePathExt {
    /// Extracts the page name from its path.
    fn page_name(&self) -> Option<Cow<str>>;
    /// Extracts the platform from the page path.
    fn page_platform(&self) -> Option<Cow<str>>;
}

impl PagePathExt for Path {
    fn page_name(&self) -> Option<Cow<str>> {
        self.file_stem().map(OsStr::to_string_lossy)
    }

    fn page_platform(&self) -> Option<Cow<str>> {
        self.parent()
            .and_then(|parent| parent.file_name().map(OsStr::to_string_lossy))
    }
}

/// Calculates the SHA256 hash and returns a hexadecimal string.
pub fn sha256_hexdigest(data: &[u8]) -> String {
    let digest = digest(&SHA256, data);
    let mut hex = String::with_capacity(64);

    for part in digest.as_ref() {
        let _ = write!(hex, "{part:02x}");
    }

    hex
}

const DAY: u64 = 86400;
const HOUR: u64 = 3600;
const MINUTE: u64 = 60;

/// Convert time in seconds to a human-readable `String`.
pub fn duration_fmt(mut secs: u64) -> String {
    let days = secs / DAY;
    secs %= DAY;
    let hours = secs / HOUR;

    if days == 0 {
        secs %= HOUR;
        let minutes = secs / MINUTE;

        if hours == 0 {
            if minutes == 0 {
                format!("{secs}s")
            } else {
                secs %= MINUTE;

                if secs == 0 {
                    format!("{minutes}min")
                } else {
                    format!("{minutes}min, {secs}s")
                }
            }
        } else if minutes == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h, {minutes}min")
        }
    } else if hours == 0 {
        format!("{days}d")
    } else {
        format!("{days}d, {hours}h")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn prepare_env(lang: Option<&str>, language: Option<&str>) {
        if let Some(lang) = lang {
            env::set_var("LANG", lang);
        } else {
            env::remove_var("LANG");
        }

        if let Some(language) = language {
            env::set_var("LANGUAGE", language);
        } else {
            env::remove_var("LANGUAGE");
        }
    }

    #[test]
    fn env_languages() {
        // This vector contains duplicates - de-dupping is done in cache.update()
        // and cache.find(), because update() requires a sorted vector, whereas
        // find() - an unsorted one.
        let mut out_vec = vec![];

        prepare_env(Some("cz"), Some("it:cz:de"));
        get_languages_from_env(&mut out_vec);
        assert_eq!(out_vec, ["it", "cz", "de", "cz"]);

        prepare_env(Some("cz"), Some("it:de:fr"));
        out_vec.clear();
        get_languages_from_env(&mut out_vec);
        assert_eq!(out_vec, ["it", "de", "fr", "cz"]);

        prepare_env(Some("it"), None);
        out_vec.clear();
        get_languages_from_env(&mut out_vec);
        assert_eq!(out_vec, ["it"]);

        prepare_env(None, Some("it:cz"));
        out_vec.clear();
        get_languages_from_env(&mut out_vec);
        assert!(out_vec.is_empty());

        prepare_env(None, None);
        out_vec.clear();
        get_languages_from_env(&mut out_vec);
        assert!(out_vec.is_empty());

        prepare_env(Some("en_US.UTF-8"), Some("de_DE.UTF-8:pl:en"));
        out_vec.clear();
        get_languages_from_env(&mut out_vec);
        assert_eq!(out_vec, ["de_DE", "de", "pl", "en", "en_US", "en"]);
    }

    #[test]
    fn sha256() {
        assert_eq!(
            sha256_hexdigest(b"This is a test."),
            "a8a2f6ebe286697c527eb35a58b5539532e9b3ae3b64d4eb0a46fb657b41562c"
        );
    }

    #[test]
    fn dur_fmt() {
        const SECOND: u64 = 1;

        assert_eq!(duration_fmt(SECOND), "1s");

        assert_eq!(duration_fmt(MINUTE), "1min");
        assert_eq!(duration_fmt(MINUTE + SECOND), "1min, 1s");

        assert_eq!(duration_fmt(HOUR), "1h");
        assert_eq!(duration_fmt(HOUR + SECOND), "1h");
        assert_eq!(duration_fmt(HOUR + MINUTE), "1h, 1min");
        assert_eq!(duration_fmt(HOUR + MINUTE + SECOND), "1h, 1min");

        assert_eq!(duration_fmt(DAY), "1d");
        assert_eq!(duration_fmt(DAY + SECOND), "1d");
        assert_eq!(duration_fmt(DAY + HOUR), "1d, 1h");
        assert_eq!(duration_fmt(DAY + HOUR + SECOND), "1d, 1h");
    }

    #[test]
    fn page_path_and_platform() {
        let p = Path::new("/home/user/.cache/tlrc/pages.lang/platform/page.md");
        assert_eq!(p.page_name(), Some("page".into()));
        assert_eq!(p.page_platform(), Some("platform".into()));
    }
}
