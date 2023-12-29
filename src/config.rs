use std::borrow::Cow;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yansi::{Color, Style};

use crate::cache::Cache;
use crate::error::{Error, ErrorKind, Result};
use crate::util::{get_languages_from_env, warnln};

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
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

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields, default)]
pub struct OutputStyle {
    pub color: OutputColor,
    pub background: OutputColor,
    pub bold: bool,
    pub underline: bool,
    pub italic: bool,
    pub dim: bool,
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StyleConfig {
    pub title: OutputStyle,
    pub description: OutputStyle,
    pub bullet: OutputStyle,
    pub example: OutputStyle,
    pub url: OutputStyle,
    pub inline_code: OutputStyle,
    pub placeholder: OutputStyle,
}

impl Default for StyleConfig {
    fn default() -> Self {
        StyleConfig {
            title: OutputStyle {
                color: OutputColor::Magenta,
                background: OutputColor::default(),
                bold: true,
                underline: false,
                italic: false,
                dim: false,
                strikethrough: false,
            },
            description: OutputStyle {
                color: OutputColor::Magenta,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: false,
                dim: false,
                strikethrough: false,
            },
            bullet: OutputStyle {
                color: OutputColor::Green,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: false,
                dim: false,
                strikethrough: false,
            },
            example: OutputStyle {
                color: OutputColor::Cyan,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: false,
                dim: false,
                strikethrough: false,
            },
            url: OutputStyle {
                color: OutputColor::Red,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: true,
                dim: false,
                strikethrough: false,
            },
            inline_code: OutputStyle {
                color: OutputColor::Yellow,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: true,
                dim: false,
                strikethrough: false,
            },
            placeholder: OutputStyle {
                color: OutputColor::Red,
                background: OutputColor::default(),
                bold: false,
                underline: false,
                italic: true,
                dim: false,
                strikethrough: false,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CacheConfig {
    /// Cache directory.
    pub dir: PathBuf,
    /// Automatically update the cache
    /// if it is older than `max_age` hours.
    pub auto_update: bool,
    /// Max cache age in hours.
    max_age: u64,
    /// Languages to download.
    pub languages: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: Cache::locate(),
            auto_update: true,
            // 2 weeks
            max_age: 24 * 7 * 2,
            languages: vec![],
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct OutputConfig {
    /// Show the page title.
    pub show_title: bool,
    /// Show the platform in the title.
    pub platform_title: bool,
    /// Show hyphens before example descriptions.
    pub show_hyphens: bool,
    /// Show a custom string instead of a hyphen.
    pub example_prefix: Cow<'static, str>,
    /// Strip empty lines from pages.
    pub compact: bool,
    /// Print pages in raw markdown.
    pub raw_markdown: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            show_title: true,
            platform_title: false,
            show_hyphens: false,
            example_prefix: Cow::Borrowed("- "),
            compact: false,
            raw_markdown: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct IndentConfig {
    pub title: usize,
    pub description: usize,
    pub bullet: usize,
    pub example: usize,
}

impl Default for IndentConfig {
    fn default() -> Self {
        Self {
            title: 2,
            description: 2,
            bullet: 2,
            example: 4,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    pub cache: CacheConfig,
    pub output: OutputConfig,
    pub indent: IndentConfig,
    pub style: StyleConfig,
}

impl Config {
    fn parse(path: &Path) -> Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(path).map_err(|e| {
            Error::new(format!("'{}': {e}", path.display())).kind(ErrorKind::Io)
        })?)?)
    }

    pub fn new(cli_config_path: Option<PathBuf>) -> Result<Self> {
        let cfg_res = if let Some(path) = cli_config_path {
            if path.is_file() {
                return Self::parse(&path);
            }

            warnln!("'{}': not a file, ignoring --config", path.display());
            Ok(Self::default())
        } else {
            let path = Self::locate();
            if path.is_file() {
                return Self::parse(&path);
            }

            Ok(Self::default())
        };

        cfg_res.map(|mut cfg| {
            if cfg.cache.languages.is_empty() {
                get_languages_from_env(&mut cfg.cache.languages);
            }
            // English pages should always be downloaded and searched.
            cfg.cache.languages.push("en".to_string());
            cfg
        })
    }

    /// Get the default path to the config file.
    pub fn locate() -> PathBuf {
        dirs::config_dir()
            .unwrap()
            .join(env!("CARGO_PKG_NAME"))
            .join("config.toml")
    }

    /// Print the default path to the config file and create the config directory.
    pub fn print_path() -> Result<()> {
        let config_path = Config::locate();
        writeln!(io::stdout(), "{}", config_path.display())?;
        fs::create_dir_all(config_path.parent().unwrap())?;
        Ok(())
    }

    /// Print the default config.
    pub fn print_default() -> Result<()> {
        let default = toml::ser::to_string_pretty(&Config::default()).unwrap();
        write!(io::stdout(), "{default}")?;
        Ok(())
    }

    /// Convert the number of hours from config to a `Duration`.
    pub const fn cache_max_age(&self) -> Duration {
        Duration::from_secs(self.cache.max_age * 60 * 60)
    }
}
