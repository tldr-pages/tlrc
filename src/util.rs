use std::env;
use std::sync::atomic::Ordering;

use yansi::{Color, Paint};

use crate::QUIET;

pub fn log(msg: &str) {
    if QUIET.load(Ordering::Relaxed) {
        return;
    }
    eprintln!("{msg}");
}

pub fn warn(msg: &str) {
    log(&format!(
        "{} {msg}",
        Paint::new("warning:").fg(Color::Yellow).bold()
    ));
}

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
            result.push(lang[..=4].to_string());
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
