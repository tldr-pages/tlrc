use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use log::{debug, warn};
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use yansi::{Color, Style};

use crate::cache::Cache;
use crate::error::{Error, ErrorKind, Result};
use crate::util;

fn hex_to_rgb<'de, D>(deserializer: D) -> std::result::Result<[u8; 3], D::Error>
where
    D: Deserializer<'de>,
{
    const HEX_ERR: &str = "6 hexadecimal digits, optionally prefixed with '#'";

    struct HexColorVisitor;
    impl Visitor<'_> for HexColorVisitor {
        type Value = [u8; 3];

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(HEX_ERR)
        }

        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let hex = v.strip_prefix('#').unwrap_or(v);

            if hex.len() != 6 {
                return Err(serde::de::Error::invalid_length(hex.len(), &HEX_ERR));
            }

            let invalid_val = |_| serde::de::Error::invalid_value(Unexpected::Str(v), &HEX_ERR);
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(invalid_val)?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(invalid_val)?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(invalid_val)?;

            Ok([r, g, b])
        }
    }

    deserializer.deserialize_str(HexColorVisitor)
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
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
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Color256(u8),
    Rgb([u8; 3]),
    #[serde(deserialize_with = "hex_to_rgb")]
    Hex([u8; 3]),
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
            OutputColor::Default => Color::Primary,
            OutputColor::BrightBlack => Color::BrightBlack,
            OutputColor::BrightRed => Color::BrightRed,
            OutputColor::BrightGreen => Color::BrightGreen,
            OutputColor::BrightYellow => Color::BrightYellow,
            OutputColor::BrightBlue => Color::BrightBlue,
            OutputColor::BrightMagenta => Color::BrightMagenta,
            OutputColor::BrightCyan => Color::BrightCyan,
            OutputColor::BrightWhite => Color::BrightWhite,
            OutputColor::Color256(c) => Color::Fixed(c),
            OutputColor::Rgb(rgb) | OutputColor::Hex(rgb) => Color::Rgb(rgb[0], rgb[1], rgb[2]),
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
        let mut style = Style::new().fg(s.color.into()).bg(s.background.into());

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
            style = style.dim();
        }
        if s.strikethrough {
            style = style.strike();
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
    /// The mirror of tldr-pages to use.
    pub mirror: Cow<'static, str>,
    /// Automatically update the cache
    /// if it is older than `max_age` hours.
    pub auto_update: bool,
    /// Perform the automatic update after the page is shown.
    pub defer_auto_update: bool,
    /// Max cache age in hours.
    max_age: u64,
    /// Languages to download.
    pub languages: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: Cache::locate(),
            mirror: Cow::Borrowed("https://github.com/tldr-pages/tldr/releases/latest/download"),
            auto_update: true,
            defer_auto_update: false,
            // 2 weeks
            max_age: 24 * 7 * 2,
            languages: vec![],
        }
    }
}

/// Defines which options should be shown in short|long placeholders (`{{[ ... ]}}`).
#[derive(Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OptionStyle {
    Short,
    #[default]
    Long,
    Both,
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
    /// Set the max line length. 0 means to use the terminal width.
    pub line_length: usize,
    /// Strip empty lines from pages.
    pub compact: bool,
    /// Display the specified options in pages wherever possible.
    pub option_style: OptionStyle,
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
            line_length: 0,
            compact: false,
            option_style: OptionStyle::default(),
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

    pub fn new(cli_config_path: Option<&Path>) -> Result<Self> {
        let cfg_res = if let Some(path) = cli_config_path {
            if path.is_file() {
                debug!("config file (from --config): {path:?}");
                Self::parse(path)
            } else {
                warn!("'{}': not a file, ignoring --config", path.display());
                Ok(Self::default())
            }
        } else {
            let path = Self::locate();
            if path.is_file() {
                debug!("config file found: {path:?}");
                Self::parse(&path)
            } else {
                debug!("{path:?}: not a file, using the default config");
                Ok(Self::default())
            }
        };

        cfg_res.map(|mut cfg| {
            if cfg.cache.languages.is_empty() {
                debug!("languages not found in config, trying from env vars");
                util::get_languages_from_env(&mut cfg.cache.languages);
            }
            // English pages should always be downloaded and searched.
            cfg.cache.languages.push("en".to_string());

            if cfg.cache.dir.starts_with("~") {
                let mut p = dirs::home_dir().unwrap();
                p.extend(cfg.cache.dir.components().skip(1));
                cfg.cache.dir = p;
            }
            cfg
        })
    }

    /// Get the default path to the config file.
    pub fn locate() -> PathBuf {
        env::var_os("TLRC_CONFIG").map_or_else(
            || {
                dirs::config_dir()
                    .unwrap()
                    .join(env!("CARGO_PKG_NAME"))
                    .join("config.toml")
            },
            PathBuf::from,
        )
    }

    /// Print the default path to the config file and create the config directory.
    pub fn print_path() -> Result<()> {
        let config_path = Config::locate();
        writeln!(io::stdout(), "{}", config_path.display())?;

        fs::create_dir_all(config_path.parent().ok_or_else(|| {
            Error::new("cannot create the config directory: the path has only one component")
        })?)?;

        Ok(())
    }

    /// Print the default config.
    pub fn print_default() -> Result<()> {
        let mut cfg = Config::default();
        let home = dirs::home_dir().unwrap();

        if cfg.cache.dir.starts_with(&home) {
            let rel_part = cfg.cache.dir.strip_prefix(&home).unwrap();
            cfg.cache.dir = Path::new("~").join(rel_part);
        }

        let cfg = toml::ser::to_string_pretty(&cfg).unwrap();
        write!(io::stdout(), "{cfg}")?;
        Ok(())
    }

    /// Convert the number of hours from config to a `Duration`.
    pub const fn cache_max_age(&self) -> Duration {
        Duration::from_secs(self.cache.max_age * 60 * 60)
    }
}
