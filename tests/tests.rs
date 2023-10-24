use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;

const TEST_PAGE: &str = "tests/data/page.md";
const TEST_PAGE_RENDER: &str = "tests/data/page-render";
const TEST_PAGE_COMPACT_RENDER: &str = "tests/data/page-compact-render";

fn tlrc() -> Command {
    let mut cmd = Command::cargo_bin("tldr").unwrap();
    cmd.args(["--config", "/dev/null"]);
    cmd
}

#[test]
fn raw_md() {
    let expected = fs::read_to_string(TEST_PAGE).unwrap();
    tlrc()
        .args(["--raw", "--render", TEST_PAGE])
        .assert()
        .stdout(expected);
}

#[test]
fn regular_render() {
    let expected = fs::read_to_string(TEST_PAGE_RENDER).unwrap();
    tlrc()
        .args(["--render", TEST_PAGE])
        .assert()
        .stdout(expected);
}

#[test]
fn compact_render() {
    let expected = fs::read_to_string(TEST_PAGE_COMPACT_RENDER).unwrap();
    tlrc()
        .args(["--compact", "--render", TEST_PAGE])
        .assert()
        .stdout(expected);
}

#[test]
fn does_not_exist() {
    tlrc()
        .args(["--render", "/some/page/that/does/not/exist.md"])
        .assert()
        .failure();
}
