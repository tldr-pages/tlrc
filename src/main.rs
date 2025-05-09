mod args;
mod cache;
mod config;
mod error;
mod output;
mod util;

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use clap::Parser;
use config::{CacheConfig, RenderConfig};

use crate::args::Cli;
use crate::cache::Cache;
use crate::config::{Config, OptionStyle};
use crate::error::{Error, Result};
use crate::output::PageRenderer;
use crate::util::init_color;

/// If this is set to true, do not print anything except pages and errors.
static QUIET: AtomicBool = AtomicBool::new(false);

fn main() -> ExitCode {
    let (cli, cfg, render) = match parse_cli_and_cfg() {
        Ok(stuff) => stuff,
        Err(e) => return e.exit_code(),
    };
    match run(cli, cfg, &render) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => e.exit_code(),
    }
}

fn parse_cli_and_cfg() -> Result<(Cli, CacheConfig, RenderConfig)> {
    let cli = Cli::parse();

    let mut cfg = Config::new(cli.config.as_ref())?;
    cfg.output.compact = !cli.no_compact && (cli.compact || cfg.output.compact);
    cfg.output.raw_markdown = !cli.no_raw && (cli.raw || cfg.output.raw_markdown);
    cfg.output.option_style = match (cli.short_options, cli.long_options) {
        (false, false) => cfg.output.option_style,
        (true, true) => OptionStyle::Both,
        (true, false) => OptionStyle::Short,
        (false, true) => OptionStyle::Long,
    };
    let render = RenderConfig {
        output: cfg.output,
        style: cfg.style,
        indent: cfg.indent,
    };
    Ok((cli, cfg.cache, render))
}

fn run(cli: Cli, cache: CacheConfig, render: &RenderConfig) -> Result<()> {
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
        return PageRenderer::print(&path, render);
    }

    let languages_are_from_cli = cli.languages.is_some();
    // We need to clone() because this vector will not be sorted,
    // unlike the one in the config.
    let cache = Cache::new(cache);

    if cli.clean_cache {
        return cache.clean();
    }

    if cli.update {
        // update() should never use languages from --language.
        return cache.update();
    }

    // only load requires mutability
    let mut cache = cache;
    cache.load(cli.offline)?;
    let cache = cache;

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
        return cache.info();
    }
    if cli.list_platforms {
        return cache.list_platforms();
    }
    if cli.list_languages {
        return cache.list_languages();
    }

    let page_name = cli.page.join("-").to_lowercase();
    let page_paths = cache.find(&page_name, cli.languages.as_deref(), platform)?;

    if page_paths.is_empty() {
        let mut e = Error::new("page not found.");
        return if languages_are_from_cli {
            e = e.describe("Try running tldr without --language.");

            if !cli.languages.is_some_and(|languages| {
                languages
                    .iter()
                    .all(|x| cache.subdir_exists(&format!("pages.{x}")))
            }) {
                e = e.describe(Error::DESC_LANG_NOT_INSTALLED);
            }

            Err(e)
        } else {
            Err(e.describe(Error::desc_page_does_not_exist()))
        };
    }

    PageRenderer::print_cache_result(&page_paths, render)?;
    cache.check_deferred_auto_update()?;
    Ok(())
}
