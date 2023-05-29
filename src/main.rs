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
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;
use is_terminal::IsTerminal;
use yansi::Paint;

use crate::args::{Cli, ColorMode};
use crate::cache::Cache;
use crate::config::Config;
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
        return Ok(());
    }

    if cli.gen_config {
        write!(
            io::stdout(),
            "{}",
            toml::ser::to_string_pretty(&Config::default()).unwrap()
        )?;
        return Ok(());
    }

    if cli.quiet {
        QUIET.store(true, Ordering::Relaxed);
    }

    init_color(&cli.color);

    let mut config = Config::new(cli.config)?;
    let cache = Cache::new(&config.cache.dir);
    config.output.compact = !cli.no_compact && (cli.compact || config.output.compact);
    config.output.raw_markdown = !cli.no_raw && (cli.raw || config.output.raw_markdown);

    let languages_are_from_cli = cli.languages.is_some();
    let languages = cli.languages.unwrap_or_else(get_languages_from_env);
    let languages_to_download = if config.cache.languages.is_empty() {
        &languages
    } else {
        &config.cache.languages
    };

    if cli.clean_cache {
        return cache.clean();
    } else if cli.update {
        return cache.update(languages_to_download);
    }

    if !cache.exists() {
        infoln!("cache is empty, downloading...");
        cache.update(languages_to_download)?;
    }

    let platform = cli.platform.unwrap_or_default();
    if cli.list {
        return cache.list_platform(platform);
    } else if cli.list_all {
        return cache.list_all();
    } else if cli.info {
        return cache.info();
    } else if let Some(path) = cli.render {
        return print_page(&path, &config.output, &config.indent, config.style);
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

    let page_path = cache
        .find(&page_name, &languages, platform)
        .map_err(|mut e| {
            if languages_are_from_cli {
                e = e.describe("Try running tldr without --language.");

                // This checks whether any language specified on the cli would not be downloaded
                // during a cache update.
                if !languages_to_download.iter().all(|x| languages.contains(x)) {
                    e = e.describe(
                        "\n\nThe language you are trying to view the page in \
                        may not be installed.\n\
                        You can run 'tldr --info' to see currently installed languages.\n\
                        Please update your config and run 'tldr --update' to install a new language.",
                    );
                }

                e
            } else {
                e.describe(format!(
                    "Try running 'tldr --update'.\n\n\
                    If the page does not exist, you can create an issue here:\n\
                    {}\n\
                    or document it yourself and create a pull request here:\n\
                    {}",
                    Paint::new("https://github.com/tldr-pages/tldr/issues").bold(),
                    Paint::new("https://github/com/tldr-pages/tldr/pulls").bold()
                ))
            }
        })?;

    print_page(&page_path, &config.output, &config.indent, config.style)
}

fn main() {
    if let Err(e) = run() {
        e.exit();
    }
}
