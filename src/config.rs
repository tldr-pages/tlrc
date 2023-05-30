use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yansi::{Color, Style};

use crate::cache::Cache;
use crate::error::{Error, ErrorKind, Result};
use crate::util::warnln;

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    #[default]
    Default,
    Color256(u8),
    Rgb([u8; 3]),
}

impl From<OutputColor> for yansi::Color {
    fn from(c: OutputColor) -> Self {
        match c {
            OutputColor::Black => Color::Black,
            OutputColor::Red => Color::Red,
            OutputColor::Green => Color::Green,
            OutputColor::Yellow => Color::Yellow,
            OutputColor::Blue => Color::Blue,
            OutputColor::Magenta => Color::Magenta,
            OutputColor::Cyan => Color::Cyan,
            OutputColor::White => Color::White,
            OutputColor::Default => Color::Default,
            OutputColor::Color256(c) => Color::Fixed(c),
            OutputColor::Rgb(rgb) => Color::RGB(rgb[0], rgb[1], rgb[2]),
        }
    }
}

// Serde doesn't support default values directly, so we need to
// wrap them in a function.
const fn bool_false() -> bool {
    false
}

const fn bool_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputStyle {
    #[serde(default)]
    pub color: OutputColor,

    #[serde(default)]
    pub background: OutputColor,

    #[serde(default = "bool_false")]
    pub bold: bool,

    #[serde(default = "bool_false")]
    pub underline: bool,

    #[serde(default = "bool_false")]
    pub italic: bool,

    #[serde(default = "bool_false")]
    pub dim: bool,

    #[serde(default = "bool_false")]
    pub strikethrough: bool,
}

impl From<OutputStyle> for yansi::Style {
    fn from(s: OutputStyle) -> Self {
        let mut style = Style::new(s.color.into()).bg(s.background.into());

        if s.bold {
            style = style.bold();
        }
        if s.italic {
            style = style.italic();
        }
        if s.underline {
            style = style.underline();
        }
        if s.dim {
            style = style.dimmed();
        }
        if s.strikethrough {
            style = style.strikethrough();
        }

        style
    }
}

const fn default_title_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Magenta,
        background: OutputColor::Default,
        bold: true,
        underline: false,
        italic: false,
        dim: false,
        strikethrough: false,
    }
}

const fn default_description_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Magenta,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: false,
        dim: false,
        strikethrough: false,
    }
}

const fn default_bullet_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Green,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: false,
        dim: false,
        strikethrough: false,
    }
}

const fn default_example_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Cyan,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: false,
        dim: false,
        strikethrough: false,
    }
}

const fn default_url_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Red,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: true,
        dim: false,
        strikethrough: false,
    }
}

const fn default_code_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Yellow,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: true,
        dim: false,
        strikethrough: false,
    }
}

const fn default_placeholder_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Red,
        background: OutputColor::Default,
        bold: false,
        underline: false,
        italic: true,
        dim: false,
        strikethrough: false,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleConfig {
    #[serde(default = "default_title_style")]
    pub title: OutputStyle,

    #[serde(default = "default_description_style")]
    pub description: OutputStyle,

    #[serde(default = "default_bullet_style")]
    pub bullet: OutputStyle,

    #[serde(default = "default_example_style")]
    pub example: OutputStyle,

    #[serde(default = "default_url_style")]
    pub url: OutputStyle,

    #[serde(default = "default_code_style")]
    pub inline_code: OutputStyle,

    #[serde(default = "default_placeholder_style")]
    pub placeholder: OutputStyle,
}

const fn default_cache_max_age() -> u64 {
    // 2 weeks
    24 * 7 * 2
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheConfig {
    /// Cache directory.
    #[serde(default = "Cache::locate")]
    pub dir: PathBuf,

    /// Automatically update the cache
    /// if it is older than `max_age` hours.
    #[serde(default = "bool_true")]
    pub auto_update: bool,

    /// Max cache age in hours.
    #[serde(default = "default_cache_max_age")]
    max_age: u64,

    /// Languages to download.
    #[serde(default = "Vec::new")]
    pub languages: Vec<String>,
}

fn hyphen() -> String {
    "- ".to_string()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    /// Show the page title.
    #[serde(default = "bool_true")]
    pub show_title: bool,

    /// Show hyphens before example descriptions.
    #[serde(default = "bool_false")]
    pub show_hyphens: bool,

    /// Show a custom string instead of a hyphen.
    #[serde(default = "hyphen")]
    pub example_prefix: String,

    /// Strip empty lines from pages.
    #[serde(default = "bool_false")]
    pub compact: bool,

    /// Print pages in raw markdown.
    #[serde(default = "bool_false")]
    pub raw_markdown: bool,
}

const fn two() -> usize {
    2
}
const fn four() -> usize {
    4
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndentConfig {
    #[serde(default = "two")]
    pub title: usize,

    #[serde(default = "two")]
    pub description: usize,

    #[serde(default = "two")]
    pub bullet: usize,

    #[serde(default = "four")]
    pub example: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub cache: CacheConfig,
    pub output: OutputConfig,
    pub indent: IndentConfig,
    pub style: StyleConfig,
}

impl Config {
    fn parse(path: &Path) -> Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(path).map_err(|e| {
            Error::new(format!("could not read the config: {e}")).kind(ErrorKind::Io)
        })?)?)
    }

    pub fn new(cli_config_path: Option<PathBuf>) -> Result<Self> {
        let config_is_from_cli = cli_config_path.is_some();
        let config_location = cli_config_path.unwrap_or_else(Self::locate);

        if config_location.is_file() {
            Self::parse(&config_location)
        } else {
            if config_is_from_cli {
                warnln!(
                    "'{}': not a file, ignoring --config",
                    config_location.display()
                );
            }
            Ok(Self::default())
        }
    }

    /// Get the default path to the config file.
    pub fn locate() -> PathBuf {
        dirs::config_dir()
            .unwrap()
            .join(clap::crate_name!())
            .join("config.toml")
    }

    /// Convert the number of hours from config to a `Duration`.
    pub const fn cache_max_age(&self) -> Duration {
        Duration::from_secs(self.cache.max_age * 60 * 60)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: CacheConfig {
                dir: Cache::locate(),
                auto_update: true,
                max_age: default_cache_max_age(),
                languages: vec![],
            },
            output: OutputConfig {
                show_title: true,
                show_hyphens: false,
                example_prefix: "- ".to_string(),
                compact: false,
                raw_markdown: false,
            },
            indent: IndentConfig {
                title: 2,
                description: 2,
                bullet: 2,
                example: 4,
            },
            style: StyleConfig {
                title: default_title_style(),
                description: default_description_style(),
                bullet: default_bullet_style(),
                example: default_example_style(),
                url: default_url_style(),
                inline_code: default_code_style(),
                placeholder: default_placeholder_style(),
            },
        }
    }
}
