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
use std::io::{self, IsTerminal};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use clap::{ColorChoice, Parser};
use yansi::Paint;

use crate::args::Cli;
use crate::cache::Cache;
use crate::config::Config;
use crate::error::{ErrorKind, Result};
use crate::output::PageRenderer;
use crate::util::{get_languages_from_env, infoln, warnln};

/// If this is set to true, do not print anything except pages and errors.
pub static QUIET: AtomicBool = AtomicBool::new(false);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => e.exit_code(),
    }
}

fn init_color(color_mode: ColorChoice) {
    #[cfg(target_os = "windows")]
    let color_support = Paint::enable_windows_ascii();
    #[cfg(not(target_os = "windows"))]
    let color_support = true;

    match color_mode {
        ColorChoice::Always => {}
        ColorChoice::Never => Paint::disable(),
        ColorChoice::Auto => {
            if !(color_support && env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal()) {
                Paint::disable();
            }
        }
    }
}

fn get_languages(config: &mut Config) -> Vec<String> {
    if config.cache.languages.is_empty() {
        get_languages_from_env()
    } else {
        // English pages should always be downloaded and searched.
        config.cache.languages.push("en".to_string());
        config.cache.languages.clone()
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.config_path {
        return Config::print_path();
    }

    if cli.gen_config {
        return Config::print_default();
    }

    if cli.quiet {
        QUIET.store(true, Relaxed);
    }

    init_color(cli.color);

    let mut config = Config::new(cli.config)?;
    let languages_are_from_cli = cli.languages.is_some();
    let mut languages = cli.languages.unwrap_or_else(|| get_languages(&mut config));
    let languages_to_download = if languages_are_from_cli {
        // update() should never use languages from `--language`.
        get_languages(&mut config)
    } else {
        // get_languages() has already been called, we need to clone() because this vector will
        // be sorted alphabetically, unlike the first one.
        languages.clone()
    };
    let cache = Cache::new(&config.cache.dir);

    if cli.clean_cache {
        return cache.clean();
    }
    if cli.update {
        return cache.update(&languages_to_download);
    }

    if !cache.exists() {
        infoln!("cache is empty, downloading...");
        cache.update(&languages_to_download)?;
    }

    if cli.list {
        return cache.list_platform(cli.platform);
    }
    if cli.list_all {
        return cache.list_all();
    }
    if cli.info {
        return cache.info();
    }

    config.output.compact = !cli.no_compact && (cli.compact || config.output.compact);
    config.output.raw_markdown = !cli.no_raw && (cli.raw || config.output.raw_markdown);
    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &config);
    }

    if config.cache.auto_update && cache.is_stale(config.cache_max_age())? {
        if cli.offline {
            warnln!("cache is stale. Run tldr without --offline to update.");
        } else {
            infoln!("cache is stale, updating...");
            cache
                .update(&languages_to_download)
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
    let page_paths = cache
        .find(&page_name, &mut languages, cli.platform)
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

    PageRenderer::print_cache_result(&page_paths, &config)
}
