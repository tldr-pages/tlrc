use std::env;

use yansi::{Color, Paint};

/// Prints a warning.
macro_rules! warnln {
    ( $( $arg:tt )*) => {
        if !$crate::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
            eprint!("{} ", yansi::Paint::new("warning:").fg(yansi::Color::Yellow).bold());
            eprintln!($($arg)*);
        }
    };
}

/// Prints a status message.
macro_rules! infoln {
    ( $( $arg:tt )*) => {
        if !$crate::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
            eprint!("{} ", yansi::Paint::new("info:").fg(yansi::Color::Cyan).bold());
            eprintln!($($arg)*);
        }
    };
}

pub(crate) use {infoln, warnln};

pub fn error(msg: &str) {
    eprintln!("{} {msg}", Paint::new("error:").fg(Color::Red).bold());
}

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
            result.push(lang[..5].to_string());
            // <language> (ll - 2 characters)
            result.push(lang[..2].to_string());
        } else if lang.len() == 2 {
            result.push(lang.to_string());
        }
    }

    result.push("en".to_string());

    result
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
