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

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use clap::Parser;

use crate::args::Cli;
use crate::cache::Cache;
use crate::config::Config;
use crate::error::{Error, ErrorKind, Result};
use crate::output::PageRenderer;
use crate::util::{get_languages, infoln, init_color, warnln};

/// If this is set to true, do not print anything except pages and errors.
pub static QUIET: AtomicBool = AtomicBool::new(false);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => e.exit_code(),
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

    config.output.compact = !cli.no_compact && (cli.compact || config.output.compact);
    config.output.raw_markdown = !cli.no_raw && (cli.raw || config.output.raw_markdown);
    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &config);
    }

    if !cache.exists() {
        infoln!("cache is empty, downloading...");
        cache.update(&languages_to_download)?;
    }

    // "macos" should be an alias of "osx".
    // Since the `macos` directory doesn't exist, this has to be changed before it
    // gets passed to cache functions (which expect directory names).
    let platform = if cli.platform == "macos" {
        "osx"
    } else {
        &cli.platform
    };

    if cli.list {
        return cache.list_platform(platform);
    }
    if cli.list_all {
        return cache.list_all();
    }
    if cli.info {
        return cache.info();
    }

    if config.cache.auto_update && cache.is_stale(config.cache_max_age())? {
        if cli.offline {
            warnln!("cache is stale. Run tldr without --offline to update.");
        } else {
            infoln!("cache is stale, updating...");
            cache
                .update(&languages_to_download)
                .map_err(|e| match e.kind {
                    ErrorKind::Download => e.describe(Error::DESC_DOWNLOAD_ERR),
                    _ => e,
                })?;
        }
    }

    let page_name = cli.page.join("-").to_lowercase();
    let page_paths = cache.find(&page_name, &mut languages, platform)?;

    if page_paths.is_empty() {
        let mut e = Error::new("page not found.");
        return if languages_are_from_cli {
            e = e.describe("Try running tldr without --language.");

            // This checks whether any language specified on the cli would not be downloaded
            // during a cache update.
            if !languages_to_download.iter().all(|x| languages.contains(x)) {
                e = e.describe(Error::DESC_LANG_NOT_INSTALLED);
            }

            Err(e)
        } else {
            Err(e.describe(Error::desc_page_does_not_exist()))
        };
    }

    PageRenderer::print_cache_result(&page_paths, &config)
}
