mod args;
mod cache;
mod config;
mod error;
mod output;
mod util;

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use clap::Parser;
use yansi::Paint;

use crate::args::Cli;
use crate::cache::Cache;
use crate::config::{Config, OptionStyle};
use crate::error::{Error, Result};
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

fn include_cli_in_config(cfg: &mut Config, cli: &Cli) {
    cfg.output.compact = !cli.no_compact && (cli.compact || cfg.output.compact);
    cfg.output.raw_markdown = !cli.no_raw && (cli.raw || cfg.output.raw_markdown);
    match (cli.short_options, cli.long_options) {
        (false, false) => {}
        (true, true) => cfg.output.option_style = OptionStyle::Both,
        (true, false) => cfg.output.option_style = OptionStyle::Short,
        (false, true) => cfg.output.option_style = OptionStyle::Long,
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

    let mut cfg = Config::new(cli.config.as_deref())?;
    include_cli_in_config(&mut cfg, &cli);

    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &cfg);
    }

    let languages_are_from_cli = cli.languages.is_some();
    // We need to clone() because this vector will not be sorted,
    // unlike the one in the config.
    let languages = cli.languages.unwrap_or_else(|| cfg.cache.languages.clone());
    let cache = Cache::new(&cfg.cache.dir);

    if cli.clean_cache {
        return cache.clean();
    }

    if cli.update {
        // update() should never use languages from --language.
        return cache.update(&cfg.cache.mirror, &mut cfg.cache.languages);
    }

    // Update after displaying the page?
    let mut update_later = false;

    if !cache.subdir_exists(cache::ENGLISH_DIR) {
        if cli.offline {
            return Err(Error::offline_no_cache());
        }
        infoln!("cache is empty, downloading...");
        cache.update(&cfg.cache.mirror, &mut cfg.cache.languages)?;
    } else if cfg.cache.auto_update && cache.age()? > cfg.cache_max_age() {
        let age = util::duration_fmt(cache.age()?.as_secs());
        let age = age.green().bold();

        if cli.offline {
            warnln!(
                "cache is stale (last update: {age} ago). Run tldr without --offline to update."
            );
        } else if cfg.cache.defer_auto_update {
            infoln!("cache is stale (last update: {age} ago), update has been deferred");
            update_later = true;
        } else {
            infoln!("cache is stale (last update: {age} ago), updating...");
            cache
                .update(&cfg.cache.mirror, &mut cfg.cache.languages)
                .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
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
        cache.list_for(platform)?;
    } else if cli.list_all {
        cache.list_all()?;
    } else if cli.info {
        cache.info(&cfg)?;
    } else if cli.list_platforms {
        cache.list_platforms()?;
    } else if cli.list_languages {
        cache.list_languages()?;
    } else {
        let page_name = cli.page.join("-").to_lowercase();
        let mut page_paths = cache.find(&page_name, &languages, platform)?;
        let forced_update_no_page = update_later && page_paths.is_empty();
        if forced_update_no_page {
            // Since the page hasn't been found and the cache is stale, disregard the defer option.
            warnln!("page not found, updating now...");
            cache
                .update(&cfg.cache.mirror, &mut cfg.cache.languages)
                .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
            page_paths = cache.find(&page_name, &languages, platform)?;
            // Reset the defer flag in order not to update twice.
            update_later = false;
        }

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
                // If the cache has been updated, don't suggest running 'tldr --update'.
                Err(e.describe(Error::desc_page_does_not_exist(!forced_update_no_page)))
            };
        }

        PageRenderer::print_cache_result(&page_paths, &cfg)?;
    }

    if update_later {
        cache
            .update(&cfg.cache.mirror, &mut cfg.cache.languages)
            .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
    }

    Ok(())
}
