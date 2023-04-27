use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yansi::{Color, Style};

use crate::cache::Cache;
use crate::error::Result;

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
pub struct OutputStyle {
    #[serde(default)]
    pub color: OutputColor,

    #[serde(default = "bool_false")]
    pub bold: bool,

    #[serde(default = "bool_false")]
    pub underline: bool,

    #[serde(default = "bool_false")]
    pub italic: bool,
}

impl From<OutputStyle> for yansi::Style {
    fn from(s: OutputStyle) -> Self {
        let mut style = Style::new(s.color.into());

        if s.bold {
            style = style.bold();
        }
        if s.italic {
            style = style.italic();
        }
        if s.underline {
            style = style.underline();
        }

        style
    }
}

const fn default_title_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Magenta,
        bold: true,
        underline: false,
        italic: false,
    }
}

const fn default_description_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Magenta,
        bold: false,
        underline: false,
        italic: false,
    }
}

const fn default_bullet_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Green,
        bold: false,
        underline: false,
        italic: false,
    }
}

const fn default_example_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Cyan,
        bold: false,
        underline: false,
        italic: false,
    }
}

const fn default_url_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Red,
        bold: false,
        underline: false,
        italic: true,
    }
}

const fn default_code_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Yellow,
        bold: false,
        underline: false,
        italic: true,
    }
}

const fn default_placeholder_style() -> OutputStyle {
    OutputStyle {
        color: OutputColor::Red,
        bold: false,
        underline: false,
        italic: true,
    }
}

#[derive(Serialize, Deserialize)]
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
    /// Languages to download. If empty, download everything.
    #[serde(default = "Vec::new")]
    pub languages: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OutputConfig {
    /// Show page title.
    #[serde(default = "bool_true")]
    pub show_title: bool,
    /// Strip empty lines from pages.
    #[serde(default = "bool_false")]
    pub compact: bool,
    /// Print pages in raw markdown.
    #[serde(default = "bool_false")]
    pub raw_markdown: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheConfig,
    pub output: OutputConfig,
    pub style: StyleConfig,
}

impl Config {
    pub fn parse(file: PathBuf) -> Result<Config> {
        Ok(toml::from_str(&fs::read_to_string(file)?)?)
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
                auto_update: false,
                max_age: default_cache_max_age(),
                languages: vec![],
            },
            output: OutputConfig {
                show_title: true,
                compact: false,
                raw_markdown: false,
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

pub fn gen_config_and_exit() -> Result<()> {
    let mut config = String::new();
    Config::default()
        .serialize(toml::Serializer::new(&mut config))
        .unwrap();
    print!("{config}");

    exit(0);
}
