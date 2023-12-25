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
use crate::util::{infoln, init_color, warnln};

/// If this is set to true, do not print anything except pages and errors.
static QUIET: AtomicBool = AtomicBool::new(false);

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

    let mut cfg = Config::new(cli.config)?;
    cfg.output.compact = !cli.no_compact && (cli.compact || cfg.output.compact);
    cfg.output.raw_markdown = !cli.no_raw && (cli.raw || cfg.output.raw_markdown);

    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &cfg);
    }

    let languages_are_from_cli = cli.languages.is_some();
    // We need to clone() because this vector will not be sorted,
    // unlike the one in the config.
    let mut languages = cli.languages.unwrap_or_else(|| cfg.cache.languages.clone());
    let cache = Cache::new(&cfg.cache.dir);

    if cli.clean_cache {
        return cache.clean();
    }

    if cli.update {
        // update() should never use languages from --language.
        return cache.update(&mut cfg.cache.languages);
    }

    if !cache.subdir_exists(cache::ENGLISH_DIR) {
        if cli.offline {
            return Err(Error::offline_no_cache());
        }
        infoln!("cache is empty, downloading...");
        cache.update(&mut cfg.cache.languages)?;
    } else if cfg.cache.auto_update && cache.age()? > cfg.cache_max_age() {
        if cli.offline {
            warnln!("cache is stale. Run tldr without --offline to update.");
        } else {
            infoln!("cache is stale, updating...");
            cache
                .update(&mut cfg.cache.languages)
                .map_err(|e| match e.kind {
                    ErrorKind::Download => e.describe(Error::DESC_DOWNLOAD_ERR),
                    _ => e,
                })?;
        }
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
        return cache.list_for(platform);
    }
    if cli.list_all {
        return cache.list_all();
    }
    if cli.info {
        return cache.info(&cfg);
    }
    if cli.list_platforms {
        return cache.list_platforms();
    }
    if cli.list_languages {
        return cache.list_languages();
    }

    let page_name = cli.page.join("-").to_lowercase();
    let page_paths = cache.find(&page_name, &mut languages, platform)?;

    if page_paths.is_empty() {
        let mut e = Error::new("page not found.");
        return if languages_are_from_cli {
            e = e.describe("Try running tldr without --language.");

            if !languages
                .iter()
                .all(|x| cache.subdir_exists(&format!("pages.{x}")))
            {
                e = e.describe(Error::DESC_LANG_NOT_INSTALLED);
            }

            Err(e)
        } else {
            Err(e.describe(Error::desc_page_does_not_exist()))
        };
    }

    PageRenderer::print_cache_result(&page_paths, &cfg)
}
