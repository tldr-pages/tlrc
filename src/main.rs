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
    let (cli, cfg) = match parse_cli_and_cfg() {
        Ok(stuff) => stuff,
        Err(e) => return e.exit_code(),
    };
    match run(cli, cfg) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => e.exit_code(),
    }
}

fn parse_cli_and_cfg() -> Result<(Cli, Config)> {
    let cli = Cli::parse();

    let mut cfg = Config::new(&cli.config)?;
    cfg.output.compact = !cli.no_compact && (cli.compact || cfg.output.compact);
    cfg.output.raw_markdown = !cli.no_raw && (cli.raw || cfg.output.raw_markdown);
    cfg.cache.optimistic_cache = cli.optimistic_cache || cfg.cache.optimistic_cache;
    cfg.output.option_style = match (cli.short_options, cli.long_options) {
        (false, false) => cfg.output.option_style,
        (true, true) => OptionStyle::Both,
        (true, false) => OptionStyle::Short,
        (false, true) => OptionStyle::Long,
    };
    Ok((cli, cfg))
}

fn run(mut cli: Cli, mut cfg: Config) -> Result<()> {
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

    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &cfg);
    }

    let languages_are_from_cli = cli.languages.is_some();
    // We need to clone() because this vector will not be sorted,
    // unlike the one in the config.
    let languages = cli
        .languages
        .clone()
        .unwrap_or_else(|| cfg.cache.languages.clone());
    let cache = Cache::new(&cfg.cache.dir);

    if cli.clean_cache {
        return cache.clean();
    }

    if cli.update {
        // update() should never use languages from --language.
        return cache.update(&cfg.cache.mirror, &mut cfg.cache.languages);
    }
    let mut should_defer_cache_update = false;

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
        } else if cfg.cache.optimistic_cache {
            should_defer_cache_update = true;
            // For optimistic cache, we'll notify the user but defer the update until after displaying the page
            warnln!(
                "cache is stale (last update: {age} ago), will defer update after cache lookup. Run without --optimistic-cache to update before lookup"
            );
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
    let page_paths = cache.find(&page_name, &languages, platform)?;

    if page_paths.is_empty() {
        if cfg.cache.optimistic_cache && should_defer_cache_update {
            warnln!("Page not found, updating cache");
            cfg.cache.optimistic_cache = false;
            cli.optimistic_cache = false;
            return run(cli, cfg);
        }
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

    PageRenderer::print_cache_result(&page_paths, &cfg)?;
    if should_defer_cache_update {
        infoln!("cache is stale, updating...");
        cache
            .update(&cfg.cache.mirror, &mut cfg.cache.languages)
            .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
    };
    Ok(())
}
