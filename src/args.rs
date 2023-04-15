use std::str::FromStr;
use std::fmt::Display;
use std::path::PathBuf;

use clap::{Parser, ArgAction};


#[derive(Clone, PartialEq)]
pub enum ColorMode {
    Always,
    Never,
    Auto,
}

impl Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Always => "always",
            Self::Never  => "never",
            Self::Auto   => "auto",
        })
    }
}

impl FromStr for ColorMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always" => Ok(Self::Always),
            "never"  => Ok(Self::Never),
            "auto"   => Ok(Self::Auto),
            _ => Err(format!("invalid value '{s}' for '--color' (possible values: always, never, auto)")),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Platform {
    Linux,
    OsX,
    Windows,
    Android,
    SunOs,
    Other,
}

impl Platform {
    #[cfg(target_os = "linux")]
    pub const fn get() -> Self { Self::Linux }
    #[cfg(target_os = "macos")]
    pub const fn get() -> Self { Self::OsX }
    #[cfg(target_os = "windows")]
    pub const fn get() -> Self { Self::Windows }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
    )))]
    pub const fn get() -> Self { Self::Other }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux"         => Ok(Self::Linux),
            "macos" | "osx" => Ok(Self::OsX),
            "windows"       => Ok(Self::Windows),
            "android"       => Ok(Self::Android),
            "sunos"         => Ok(Self::SunOs),
            _ => Err(format!("invalid platform '{s}' (possible values: linux, macos, osx, windows, android, sunos)'"))
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Linux   => "linux",
            Self::OsX     => "osx",
            Self::Windows => "windows",
            Self::Android => "android",
            Self::SunOs   => "sunos",
            Self::Other   => "other",
        })
    }
}

#[derive(Parser)]
#[command(
    arg_required_else_help = true,
    about,
    version,
    disable_version_flag = true,
    help_template = "{before-help}{name} {version}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}",
)]
pub struct Cli {
    /// The tldr page to show.
    #[arg(group = "operations")]
    pub page: Vec<String>,

    /// Update the cache.
    #[arg(short, long, group = "operations")]
    pub update: bool,

    /// List all pages in the current platform.
    #[arg(short, long, group = "operations")]
    pub list: bool,

    /// Render the specified markdown file.
    #[arg(short, long, group = "operations", value_name = "FILE")]
    pub render: Option<PathBuf>,

    /// Clean the cache.
    #[arg(long, group = "operations")]
    pub clean_cache: bool,

    /// Print the default config to stdout and exit.
    #[arg(long, group = "operations")]
    pub gen_config: bool,

    /// Print the default config path.
    #[arg(long, group = "operations")]
    pub config_path: bool,

    /// Specify the platform to use [linux, macos/osx, windows, android, sunos].
    #[arg(short, long)]
    pub platform: Option<Platform>,

    /// Specify the languages to use.
    #[arg(short = 'L', long = "language", value_name = "LANGUAGE")]
    pub languages: Option<Vec<String>>,

    /// Do not update the cache, even if it is stale.
    #[arg(short, long)]
    pub offline: bool,

    /// Supress status messages.
    #[arg(short, long)]
    pub quiet: bool,

    /// Specify when to enable color [always, never, auto].
    #[arg(long, value_name = "WHEN", default_value_t = ColorMode::Auto)]
    pub color: ColorMode,

    /// Specify an alternative path to the config file.
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Print version.
    #[arg(short, long, action = ArgAction::Version)]
    pub version: (),
}
