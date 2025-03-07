use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;

const TEST_PAGE: &str = "tests/data/page.md";
const TEST_PAGE_OPTION_PLACEHOLDERS: &str = "tests/data/option-placeholder.md";
const TEST_PAGE_LINE_WRAPPING: &str = "tests/data/line-wrapping.md";

const TEST_PAGE_RENDER: &str = "tests/data/page-render";
const TEST_PAGE_COMPACT_RENDER: &str = "tests/data/page-compact-render";
const TEST_PAGE_LINE_WRAPPING_RENDER: &str = "tests/data/line-wrapping-render";

const CONFIG_LINE_WRAPPING: &str = "tests/data/line-wrapping.toml";
const CONFIG_DEFAULT: &str = "/dev/null";

fn tlrc(cfg: &str, page: &str) -> Command {
    let mut cmd = Command::cargo_bin("tldr").unwrap();
    cmd.args(["--config", cfg, "--render", page]);
    cmd
}

#[test]
fn raw_md() {
    let expected = fs::read_to_string(TEST_PAGE).unwrap();
    tlrc(CONFIG_DEFAULT, TEST_PAGE)
        .args(["--raw"])
        .assert()
        .stdout(expected);
}

#[test]
fn regular_render() {
    let expected = fs::read_to_string(TEST_PAGE_RENDER).unwrap();
    tlrc(CONFIG_DEFAULT, TEST_PAGE).assert().stdout(expected);
}

#[test]
fn compact_render() {
    let expected = fs::read_to_string(TEST_PAGE_COMPACT_RENDER).unwrap();
    tlrc(CONFIG_DEFAULT, TEST_PAGE)
        .args(["--compact"])
        .assert()
        .stdout(expected);
}

#[test]
fn does_not_exist() {
    tlrc(CONFIG_DEFAULT, "/some/page/that/does/not/exist.md")
        .assert()
        .failure();
}

#[test]
fn short_opts() {
    tlrc(CONFIG_DEFAULT, TEST_PAGE_OPTION_PLACEHOLDERS)
        .args(["--short-options"])
        .assert()
        .stdout("    foo -s\n\n");
}

#[test]
fn long_opts() {
    tlrc(CONFIG_DEFAULT, TEST_PAGE_OPTION_PLACEHOLDERS)
        .args(["--long-options"])
        .assert()
        .stdout("    foo --long\n\n");
}

#[test]
fn both_opts() {
    tlrc(CONFIG_DEFAULT, TEST_PAGE_OPTION_PLACEHOLDERS)
        .args(["--short-options", "--long-options"])
        .assert()
        .stdout("    foo [-s|--long]\n\n");
}

#[test]
fn wrap_lines() {
    let expected = fs::read_to_string(TEST_PAGE_LINE_WRAPPING_RENDER).unwrap();
    tlrc(CONFIG_LINE_WRAPPING, TEST_PAGE_LINE_WRAPPING)
        .assert()
        .stdout(expected);
}
