use std::collections::HashSet;
use std::env;
use std::hash::Hash;

/// Prints a warning.
macro_rules! warnln {
    ( $( $arg:tt )* ) => {
        if !$crate::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
            use std::io::Write;
            let mut stderr = std::io::stderr().lock();
            write!(stderr, "{} ", yansi::Paint::new("warning:").fg(yansi::Color::Yellow).bold())?;
            writeln!(stderr, $($arg)*)?;
        }
    };
}

/// Prints a status message.
macro_rules! infoln {
    ( $( $arg:tt )* ) => {
        if !$crate::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
            use std::io::Write;
            let mut stderr = std::io::stderr().lock();
            write!(stderr, "{} ", yansi::Paint::new("info:").fg(yansi::Color::Cyan).bold())?;
            writeln!(stderr, $($arg)*)?;
        }
    };
}

pub(crate) use {infoln, warnln};

pub fn get_languages_from_env() -> Vec<String> {
    // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#language

    let var_lang = env::var("LANG").ok();
    let var_language = env::var("LANGUAGE").ok();

    if var_lang.is_none() {
        return vec!["en".to_string()];
    }

    let var_lang = var_lang.unwrap();
    let var_language = var_language.as_deref();

    let mut result = vec![];
    let languages = var_language
        .unwrap_or("")
        .split(':')
        .chain([var_lang.as_str()]);

    for lang in languages {
        if lang.len() >= 5 && lang.chars().nth(2) == Some('_') {
            // <language>_<country> (ll_CC - 5 characters)
            result.push(&lang[..5]);
            // <language> (ll - 2 characters)
            result.push(&lang[..2]);
        } else if lang.len() == 2 {
            result.push(lang);
        }
    }

    result.push("en");
    result.dedup_nosort();

    result.into_iter().map(String::from).collect()
}

/// Convert language codes to directory names in the cache.
pub fn languages_to_langdirs(languages: &[String]) -> Vec<String> {
    languages
        .iter()
        .map(|lang| {
            if lang == "en" {
                "pages".to_string()
            } else {
                format!("pages.{lang}")
            }
        })
        .collect()
}

trait Dedup {
    /// Deduplicate a vector in place preserving the order of elements.
    fn dedup_nosort(&mut self);
}

impl<T> Dedup for Vec<T>
where
    T: Hash + Eq + Copy,
{
    fn dedup_nosort(&mut self) {
        let mut set = HashSet::new();
        self.retain(|x| set.insert(*x));
    }
}
