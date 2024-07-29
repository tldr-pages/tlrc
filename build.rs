/// This build script generates a version string.
use std::env;
use std::process;

/// The version of the tldr client specification being implemented.
const CLIENT_SPEC: &str = "2.2";

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

/// Get `CARGO_PKG_VERSION` and the client spec version.
fn pkgver_and_spec() -> String {
    format!(
        "v{} (implementing the tldr client specification v{CLIENT_SPEC})",
        env!("CARGO_PKG_VERSION")
    )
}

fn main() {
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
}
