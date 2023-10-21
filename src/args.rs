use std::fmt::Display;
use std::path::PathBuf;

use clap::{ArgAction, ColorChoice, Parser, ValueEnum};

#[derive(Copy, Clone, PartialEq, ValueEnum, Default)]
pub enum Platform {
    #[cfg_attr(target_os = "linux", default)]
    Linux,

    #[value(name = "osx", alias = "macos")]
    #[cfg_attr(target_os = "macos", default)]
    OsX,

    #[value(name = "openbsd")]
    #[cfg_attr(target_os = "openbsd", default)]
    OpenBsd,

    #[cfg_attr(target_os = "windows", default)]
    Windows,

    #[cfg_attr(target_os = "android", default)]
    Android,

    #[value(name = "sunos")]
    SunOs,

    #[cfg_attr(
        not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "openbsd",
            target_os = "windows",
            target_os = "android"
        )),
        default
    )]
    Common,
}

impl Platform {
    pub fn iterator() -> impl Iterator<Item = Platform> {
        use self::Platform::{Android, Linux, OpenBsd, OsX, SunOs, Windows};
        [Linux, OsX, OpenBsd, Windows, Android, SunOs].into_iter()
    }
}

// These are the directory names.
impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linux => "linux",
            Self::OsX => "osx",
            Self::OpenBsd => "openbsd",
            Self::Windows => "windows",
            Self::Android => "android",
            Self::SunOs => "sunos",
            Self::Common => "common",
        }
        .fmt(f)
    }
}

#[derive(Parser)]
#[command(
    arg_required_else_help = true,
    about,
    // VERSION_STRING is generated and set in the build script.
    // A fallback must be set here because this file is included as a module
    // in build.rs to generate completions, and it will refuse to compile
    // (the variable is not present yet in the build script).
    version = option_env!("VERSION_STRING").unwrap_or(env!("CARGO_PKG_VERSION")),
    disable_version_flag = true,
    after_help = "See 'man tldr' or https://acuteenvy.github.io/tlrc for more information.",
    help_template = "{before-help}{name} {version}\n\
    {about-with-newline}\n\
    {usage-heading} {usage}\n\n\
    {all-args}{after-help}"
)]
pub struct Cli {
    /// The tldr page to show.
    #[arg(group = "operations", required = true)]
    pub page: Vec<String>,

    /// Update the cache.
    #[arg(short, long, group = "operations")]
    pub update: bool,

    /// List all pages in the current platform.
    #[arg(short, long, group = "operations")]
    pub list: bool,

    /// List all pages.
    #[arg(short = 'a', long, group = "operations")]
    pub list_all: bool,

    /// Show cache information (installed languages and the number of pages).
    #[arg(short, long, group = "operations")]
    pub info: bool,

    /// Render the specified markdown file.
    #[arg(short, long, group = "operations", value_name = "FILE")]
    pub render: Option<PathBuf>,

    /// Clean the cache.
    #[arg(long, group = "operations")]
    pub clean_cache: bool,

    /// Print the default config.
    #[arg(long, group = "operations")]
    pub gen_config: bool,

    /// Print the default config path and create the config directory.
    #[arg(long, group = "operations")]
    pub config_path: bool,

    /// Specify the platform to use.
    #[arg(short, long, default_value_t = Platform::default())]
    pub platform: Platform,

    /// Specify the languages to use.
    #[arg(short = 'L', long = "language", value_name = "LANGUAGE")]
    pub languages: Option<Vec<String>>,

    /// Do not update the cache, even if it is stale.
    #[arg(short, long)]
    pub offline: bool,

    /// Strip empty lines from output.
    #[arg(short, long)]
    pub compact: bool,

    /// Do not strip empty lines from output (overrides --compact).
    #[arg(long)]
    pub no_compact: bool,

    /// Print pages in raw markdown instead of rendering them.
    #[arg(short = 'R', long)]
    pub raw: bool,

    /// Render pages instead of printing raw file contents (overrides --raw).
    #[arg(long)]
    pub no_raw: bool,

    /// Supress status messages.
    #[arg(short, long)]
    pub quiet: bool,

    /// Specify when to enable color.
    #[arg(long, value_name = "WHEN", default_value_t = ColorChoice::default())]
    pub color: ColorChoice,

    /// Specify an alternative path to the config file.
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Print version.
    #[arg(short, long, action = ArgAction::Version)]
    version: (),
}
