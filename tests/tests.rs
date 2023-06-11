use std::fs;
use std::process::Command;

const TEST_PAGE: &str = "tests/data/page.md";
const TEST_PAGE_RENDER: &str = "tests/data/page-render";
const TEST_PAGE_COMPACT_RENDER: &str = "tests/data/page-compact-render";

pub fn tlrc(args: &[&str]) -> String {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--", "--config", "/dev/null"]);
    cmd.args(args);

    let output = cmd.output().unwrap();

    if !output.status.success() {
        let err = String::from_utf8(output.stderr).unwrap();
        panic!("{err}");
    }

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn raw_md() {
    let expected = fs::read_to_string(TEST_PAGE).unwrap();
    let out = tlrc(&["--raw", "--render", TEST_PAGE]);

    assert_eq!(expected, out);
}

#[test]
fn regular_render() {
    let expected = fs::read_to_string(TEST_PAGE_RENDER).unwrap();
    let out = tlrc(&["--render", TEST_PAGE]);

    assert_eq!(expected, out);
}

#[test]
fn compact_render() {
    let expected = fs::read_to_string(TEST_PAGE_COMPACT_RENDER).unwrap();
    let out = tlrc(&["--compact", "--render", TEST_PAGE]);

    assert_eq!(expected, out);
}

#[test]
#[should_panic]
fn does_not_exist() {
    tlrc(&["--render", "/some/page/that/does/not/exist.md"]);
}
