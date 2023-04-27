use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yansi::{Color, Style};

use crate::cache::Cache;
use crate::error::Result;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct OutputStyle {
    pub color: OutputColor,
    pub bold: bool,
    pub underline: bool,
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

#[derive(Serialize, Deserialize)]
pub struct StyleConfig {
    pub title: OutputStyle,
    pub description: OutputStyle,
    pub bullet: OutputStyle,
    pub example: OutputStyle,
    pub url: OutputStyle,
    pub inline_code: OutputStyle,
    pub placeholder: OutputStyle,
}

#[derive(Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache directory.
    pub dir: PathBuf,
    /// Automatically update the cache
    /// if it is older than `max_age` hours.
    pub auto_update: bool,
    /// Max cache age in hours.
    max_age: u64,
    /// Languages to download. If empty, download everything.
    pub languages: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OutputConfig {
    /// Show page title.
    pub show_title: bool,
    /// Strip empty lines from pages.
    pub compact: bool,
    /// Print pages in raw markdown.
    pub raw_markdown: bool,
}

#[derive(Serialize, Deserialize)]
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
                auto_update: true,
                // 2 weeks
                max_age: 24 * 7 * 2,
                languages: vec![],
            },
            output: OutputConfig {
                show_title: true,
                compact: false,
                raw_markdown: false,
            },
            style: StyleConfig {
                title: OutputStyle {
                    color: OutputColor::Magenta,
                    bold: true,
                    underline: false,
                    italic: false,
                },
                description: OutputStyle {
                    color: OutputColor::Magenta,
                    bold: false,
                    underline: false,
                    italic: false,
                },
                bullet: OutputStyle {
                    color: OutputColor::Green,
                    bold: false,
                    underline: false,
                    italic: false,
                },
                example: OutputStyle {
                    color: OutputColor::Cyan,
                    bold: false,
                    underline: false,
                    italic: false,
                },
                url: OutputStyle {
                    color: OutputColor::Red,
                    bold: false,
                    underline: false,
                    italic: true,
                },
                inline_code: OutputStyle {
                    color: OutputColor::Yellow,
                    bold: false,
                    underline: false,
                    italic: true,
                },
                placeholder: OutputStyle {
                    color: OutputColor::Red,
                    bold: false,
                    underline: false,
                    italic: true,
                },
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
