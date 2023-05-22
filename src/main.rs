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
use std::io::Write;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;
use is_terminal::IsTerminal;
use yansi::Paint;

use crate::args::{Cli, ColorMode, Platform};
use crate::cache::Cache;
use crate::config::{gen_config_and_exit, Config};
use crate::error::{ErrorKind, Result};
use crate::output::print_page;
use crate::util::{get_languages_from_env, infoln, warnln};

/// If this is set to true, do not print anything except pages and errors.
pub static QUIET: AtomicBool = AtomicBool::new(false);

fn init_color(color_mode: &ColorMode) {
    #[cfg(target_os = "windows")]
    let color_support = yansi::Paint::enable_windows_ascii();
    #[cfg(not(target_os = "windows"))]
    let color_support = true;

    match color_mode {
        ColorMode::Always => {}
        ColorMode::Never => yansi::Paint::disable(),
        ColorMode::Auto => {
            if !(color_support && env::var("NO_COLOR").is_err() && io::stdout().is_terminal()) {
                yansi::Paint::disable();
            }
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.config_path {
        writeln!(io::stdout(), "{}", Config::locate().display())?;
        exit(0);
    }

    if cli.gen_config {
        gen_config_and_exit()?;
    }

    if cli.quiet {
        QUIET.store(true, Ordering::Relaxed);
    }

    init_color(&cli.color);

    let config = Config::new(cli.config)?;
    let cache = Cache::new(&config.cache.dir);

    let languages_are_from_cli = cli.languages.is_some();
    let languages = cli.languages.unwrap_or_else(get_languages_from_env);
    let languages_to_download = if config.cache.languages.is_empty() {
        &languages
    } else {
        &config.cache.languages
    };

    if cli.clean_cache {
        cache.clean()?;
        exit(0);
    } else if cli.update {
        cache.update(languages_to_download)?;
        exit(0);
    } else if cli.list_all {
        cache.list_all()?;
        exit(0);
    }

    if !cache.exists() {
        infoln!("cache is empty, downloading...");
        cache.update(languages_to_download)?;
    }

    let platform = cli.platform.unwrap_or_else(Platform::get);
    if cli.list {
        cache.list_platform(&platform)?;
        exit(0);
    } else if let Some(path) = cli.render {
        let page = fs::read_to_string(path)?;
        print_page(&page, &config.output, config.style)?;
        exit(0);
    }

    if config.cache.auto_update && cache.is_stale(&config.cache_max_age())? {
        if cli.offline {
            warnln!("cache is stale. Run tldr without --offline to update.");
        } else {
            infoln!("cache is stale, updating...");
            cache
                .update(languages_to_download)
                .map_err(|e| match e.kind {
                    ErrorKind::Download => e.describe(
                        "\n\nA download error occurred. \
                        To skip updating the cache, run tldr with --offline.",
                    ),
                    _ => e,
                })?;
        }
    }

    let page_name = cli.page.join("-").to_lowercase();

    let page_path = cache.find(&page_name, &languages, &platform).map_err(|e| {
        if languages_are_from_cli {
            e.describe("Try running tldr without --language.")
        } else {
            e.describe(format!(
                "Please run 'tldr --update'.\n\n\
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
    )
}

fn main() {
    if let Err(e) = run() {
        e.exit();
    }
}
