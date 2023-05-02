#![warn(unused)]
#![warn(clippy::all, clippy::pedantic, clippy::style)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::struct_excessive_bools)]

mod args;
mod cache;
mod config;
mod error;
mod output;
mod util;

use std::env;
use std::fs;
use std::io;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;
use is_terminal::IsTerminal;
use yansi::Paint;

use crate::args::{Cli, ColorMode, Platform};
use crate::cache::Cache;
use crate::config::{gen_config_and_exit, Config};
use crate::error::{Error, Result};
use crate::output::print_page;
use crate::util::{get_languages_from_env, log, warn};

/// If this is set to true, do not print anything except pages and errors.
pub static QUIET: AtomicBool = AtomicBool::new(false);

#[allow(clippy::too_many_lines)]
fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.config_path {
        println!("{}", Config::locate().display());
        exit(0);
    }

    if cli.gen_config {
        gen_config_and_exit()?;
    }

    if cli.quiet {
        QUIET.store(true, Ordering::Relaxed);
    }

    #[cfg(target_os = "windows")]
    let color_support = yansi::Paint::enable_windows_ascii();
    #[cfg(not(target_os = "windows"))]
    let color_support = true;

    match cli.color {
        ColorMode::Always => {}
        ColorMode::Never => yansi::Paint::disable(),
        ColorMode::Auto => {
            if !(color_support && env::var("NO_COLOR").is_err() && io::stdout().is_terminal()) {
                yansi::Paint::disable();
            }
        }
    }

    let config_is_from_cli = cli.config.is_some();
    let config_location = cli.config.unwrap_or_else(Config::locate);

    let config = if config_location.is_file() {
        Config::parse(config_location)?
    } else {
        if config_is_from_cli {
            warn(&format!(
                "'{}': not a file, ignoring --config",
                config_location.display()
            ));
        }
        Config::default()
    };
    let cache = Cache::new(&config.cache.dir);

    if cli.clean_cache {
        cache.clean()?;
        exit(0);
    } else if cli.update {
        cache.update(&config.cache.languages)?;
        exit(0);
    } else if cli.list_all {
        cache.list_all()?;
        exit(0);
    }

    if !cache.exists() {
        log("Cache is empty, downloading...");
        cache.update(&config.cache.languages)?;
    }

    let platform = cli.platform.unwrap_or_else(Platform::get);
    if cli.list {
        cache.list_platform(&platform)?;
        exit(0);
    } else if let Some(path) = cli.render {
        let page = fs::read_to_string(path)?;
        print_page(&page, &config.output, config.style);
        exit(0);
    }

    if cli.page.is_empty() {
        return Err(Error::Argument("page not specified".to_string()));
    }

    if config.cache.auto_update && cache.is_stale(&config.cache_max_age())? {
        if cli.offline {
            warn("cache is stale. Run tldr without --offline to update.");
        } else {
            log("Cache is stale, updating...");
            cache.update(&config.cache.languages).map_err(|e| {
                match e {
                    Error::Download(desc) => {
                        Error::Download(format!("{desc}\n\
                        A download error occurred. To skip updating the cache, run tldr with --offline."))
                    },
                    _ => e,
                }
            })?;
        }
    }

    let languages_are_from_cli = cli.languages.is_some();
    let languages = cli.languages.unwrap_or_else(get_languages_from_env);
    let page_name = cli.page.join("-").to_lowercase();

    let page_path = cache.find(&page_name, &languages, &platform).map_err(|e| {
        if languages_are_from_cli {
            Error::Msg(format!("{e} Try running tldr without --language."))
        } else {
            Error::Msg(format!(
                "{e} Please run 'tldr --update'.\n\n\
            If you want to request creation of that page, you can file an issue here:\n\
            {}\n\
            or document it yourself and create a pull request here:\n\
            {}",
                Paint::new("https://github.com/tldr-pages/tldr/issues").bold(),
                Paint::new("https://github/com/tldr-pages/tldr/pulls").bold()
            ))
        }
    })?;

    print_page(
        &fs::read_to_string(page_path)?,
        &config.output,
        config.style,
    );

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        e.exit();
    }
}
