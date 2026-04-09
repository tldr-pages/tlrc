mod args;
mod cache;
mod config;
mod error;
mod output;
mod util;

use std::process::ExitCode;

use clap::Parser;
use log::{info, warn};
use yansi::Paint;

use crate::args::Cli;
use crate::cache::Cache;
use crate::config::{Config, OptionStyle, OutputMode};
use crate::error::{Error, Result};
use crate::output::PageRenderer;
use crate::util::{Logger, init_color};

const DEFAULT_PLATFORM: &str = if cfg!(target_os = "linux") {
    "linux"
} else if cfg!(target_os = "macos") {
    "osx"
} else if cfg!(target_os = "windows") {
    "windows"
} else if cfg!(target_os = "freebsd") {
    "freebsd"
} else if cfg!(target_os = "openbsd") {
    "openbsd"
} else if cfg!(target_os = "netbsd") {
    "netbsd"
} else if cfg!(target_os = "android") {
    "android"
} else {
    "common"
};

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_color(cli.color);
    Logger::init(cli.quiet, cli.verbose);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => e.exit_code(),
    }
}

fn include_cli_in_config(cfg: &mut Config, cli: &Cli) {
    if cli.compact {
        warn!(
            "--compact is deprecated.\nPlease use output.mode = \"very_compact\" in the config instead"
        );
        cfg.output.mode = OutputMode::VeryCompact;
    }
    if cli.no_compact {
        warn!(
            "--no-compact is deprecated.\nPlease use output.mode = \"normal\" in the config instead"
        );
        if cfg.output.mode == OutputMode::VeryCompact {
            cfg.output.mode = OutputMode::Normal;
        }
    }

    if cli.raw {
        cfg.output.mode = OutputMode::Raw;
    }
    if cli.no_raw {
        warn!("--no-raw is deprecated.\nPlease use output.mode = \"normal\" in the config instead");
        if cfg.output.mode == OutputMode::Raw {
            cfg.output.mode = OutputMode::Normal;
        }
    }

    cfg.output.edit_link |= cli.edit;

    match (cli.short_options, cli.long_options) {
        (false, false) => {}
        (true, true) => cfg.output.option_style = OptionStyle::Both,
        (true, false) => cfg.output.option_style = OptionStyle::Short,
        (false, true) => cfg.output.option_style = OptionStyle::Long,
    }
}

#[derive(PartialEq)]
enum UpdateState {
    Done,
    NotDone,
    Deferred,
}

fn run(cli: Cli) -> Result<()> {
    if cli.config_path {
        return Config::print_path();
    }

    if cli.gen_config {
        return Config::print_default();
    }

    let mut cfg = Config::new(cli.config.as_deref())?;
    include_cli_in_config(&mut cfg, &cli);

    if let Some(path) = cli.render {
        return PageRenderer::print(&path, &cfg);
    }

    // This is needed later to print a different error message if --language was used.
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

    let mut ustate = UpdateState::NotDone;

    if !cache.subdir_exists(cache::ENGLISH_DIR) {
        if cli.offline {
            return Err(Error::offline_no_cache());
        }
        info!("cache is empty, downloading...");
        cache
            .update(&cfg.cache.mirror, &mut cfg.cache.languages)
            .map_err(|e| e.describe(Error::DESC_NO_INTERNET))?;
        ustate = UpdateState::Done;
    } else if cfg.cache.auto_update && cache.age()? > cfg.cache_max_age() {
        let age = util::duration_fmt(cache.age()?.as_secs());
        let age = age.green().bold();

        if cli.offline {
            warn!("cache is stale (last update: {age} ago). Run tldr without --offline to update.");
        } else if cfg.cache.defer_auto_update {
            info!("cache is stale (last update: {age} ago), update has been deferred");
            ustate = UpdateState::Deferred;
        } else {
            info!("cache is stale (last update: {age} ago), updating...");
            cache
                .update(&cfg.cache.mirror, &mut cfg.cache.languages)
                .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
            ustate = UpdateState::Done;
        }
    }

    let platform = match cli.platform.as_deref() {
        // "macos" should be an alias of "osx".
        // Since the `macos` directory doesn't exist, this has to be changed before it
        // gets passed to cache functions (which expect directory names).
        Some("macos") => "osx",
        Some(p) => p,
        None => DEFAULT_PLATFORM,
    };

    if cli.list {
        cache.list_for(platform)?;
    } else if cli.list_all {
        cache.list_all()?;
    } else if let Some(query) = cli.search {
        // All platforms should be searched when `-p` isn't used.
        let search_plat = cli.platform.as_deref().map(|_| platform);
        cache.search(&query, search_plat, &languages, languages_are_from_cli)?;
    } else if cli.info {
        cache.info(&cfg)?;
    } else if cli.list_platforms {
        cache.list_platforms()?;
    } else if cli.list_languages {
        cache.list_languages()?;
    } else {
        let page_name = cli.page.join("-").to_lowercase();
        let mut page_paths = cache.find(&page_name, &languages, platform)?;

        if ustate == UpdateState::Deferred && page_paths.is_empty() {
            // Since the page hasn't been found and the cache is stale, disregard the defer option.
            warn!("page not found, updating now...");
            cache
                .update(&cfg.cache.mirror, &mut cfg.cache.languages)
                .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
            page_paths = cache.find(&page_name, &languages, platform)?;
            ustate = UpdateState::Done;
        }

        if page_paths.is_empty() {
            let e = Error::new("page not found.");
            return if languages_are_from_cli {
                Err(e.describe(Error::TRY_NO_EXPLICIT_LANGUAGE))
            } else {
                // If the cache has been updated, don't suggest running 'tldr --update'.
                let suggest_update = ustate != UpdateState::Done;
                Err(e.describe(Error::desc_page_does_not_exist(suggest_update)))
            };
        }

        PageRenderer::print_cache_result(&page_paths, &cfg)?;
    }

    if ustate == UpdateState::Deferred {
        cache
            .update(&cfg.cache.mirror, &mut cfg.cache.languages)
            .map_err(|e| e.describe(Error::DESC_AUTO_UPDATE_ERR))?;
    }

    Ok(())
}
