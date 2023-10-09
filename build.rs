/// This build script generates completions using clap_complete and a version string.
#[allow(dead_code)]
#[path = "src/args.rs"]
mod args;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::process;

use clap::CommandFactory;
use clap_complete::{shells, Generator};

use crate::args::Cli;

/// The version of the tldr client specification being implemented.
const CLIENT_SPEC: &str = "1.5";

fn gen_completions<G, D>(gen: G, cmd: &mut clap::Command, dir: D) -> Result<(), io::Error>
where
    G: Generator,
    D: Into<OsString>,
{
    clap_complete::generate_to(gen, cmd, "tldr", dir)?;
    Ok(())
}

fn is_debug_build() -> bool {
    env::var("PROFILE").unwrap() == "debug"
}

/// Get the short hash of the latest commit.
fn commit_hash() -> Option<String> {
    let result = process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    result.ok().and_then(|output| {
        let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    })
}

/// Get CARGO_PKG_VERSION and the client spec version.
fn pkgver_and_spec() -> String {
    format!(
        "v{} (implementing the tldr client specification v{CLIENT_SPEC})",
        env!("CARGO_PKG_VERSION")
    )
}

fn main() -> Result<(), io::Error> {
    let completion_dir = env::var("COMPLETION_DIR")
        .or_else(|_| env::var("OUT_DIR"))
        .unwrap();

    fs::create_dir_all(&completion_dir)?;

    let cmd = &mut Cli::command();
    gen_completions(shells::Bash, cmd, &completion_dir)?;
    gen_completions(shells::Zsh, cmd, &completion_dir)?;
    gen_completions(shells::Fish, cmd, &completion_dir)?;
    gen_completions(shells::PowerShell, cmd, &completion_dir)?;

    let ver = if is_debug_build() {
        if let Some(hash) = commit_hash() {
            format!("{} - debug build ({hash})", pkgver_and_spec())
        } else {
            // If git is not available, proceed with the compilation without the commit hash.
            format!("{} - debug build", pkgver_and_spec())
        }
    } else {
        // Same for release builds.
        pkgver_and_spec()
    };

    // Put the version string inside an environment variable during the build.
    println!("cargo:rustc-env=VERSION_STRING={ver}");

    Ok(())
}
