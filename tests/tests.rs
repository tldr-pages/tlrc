use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::tempdir;

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

fn run_git<P: AsRef<Path>>(cwd: P, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {:?} failed", args);
}

fn init_remote_with_page<P: AsRef<Path>>(base: P, page_content: &str) -> String {
    let base = base.as_ref();
    let remote = base.join("remote.git");
    let seed = base.join("seed");
    fs::create_dir_all(&seed).unwrap();
    run_git(base, &["init", "--bare", remote.to_str().unwrap()]);
    run_git(base, &["init", seed.to_str().unwrap()]);
    run_git(
        &seed,
        &["config", "user.email", "tldr-tests@example.invalid"],
    );
    run_git(&seed, &["config", "user.name", "tlrc tests"]);
    fs::create_dir_all(seed.join("pages/common")).unwrap();
    fs::write(seed.join("pages/common/foo.md"), page_content).unwrap();
    run_git(&seed, &["add", "."]);
    run_git(&seed, &["commit", "-m", "initial"]);
    run_git(&seed, &["branch", "-M", "main"]);
    run_git(
        &seed,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run_git(&seed, &["push", "-u", "origin", "main"]);
    remote.to_string_lossy().into_owned()
}

fn make_cfg<P: AsRef<Path>>(path: P, cache: P, taps: &str) {
    fs::write(
        path,
        format!(
            r#"[cache]
dir = "{}"
auto_update = false
languages = ["en"]

{taps}
"#,
            cache.as_ref().display()
        ),
    )
    .unwrap();
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

#[test]
fn append_personal_examples() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(cache_dir.join("pages.en/linux")).unwrap();
    fs::create_dir_all(cache_dir.join("pages.en/common")).unwrap();
    fs::create_dir_all(cache_dir.join("taps/personal/pages/common")).unwrap();

    fs::write(
        cache_dir.join("pages.en/linux/foo.md"),
        "# foo\n\n> Test page.\n\n- Official example:\n\n`foo --official`\n",
    )
    .unwrap();
    fs::write(
        cache_dir.join("taps/personal/pages/common/foo.md"),
        "# foo\n\n> Personal page.\n\n- Personal example:\n\n`foo --personal`\n",
    )
    .unwrap();

    let cfg_path = tmp.path().join("config.toml");
    fs::write(
        &cfg_path,
        format!(
            r#"[cache]
dir = "{}"
auto_update = false
languages = ["en"]

[[taps]]
name = "personal"
url = "https://example.com/personal.git"
enabled = true
"#,
            cache_dir.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("tldr").unwrap();
    cmd.args([
        "--config",
        cfg_path.to_str().unwrap(),
        "--platform",
        "linux",
        "foo",
    ]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Personal additions"));
    assert!(stdout.contains("foo --personal"));
    assert!(!stdout.contains("Personal page."));
}

#[test]
fn raw_mode_does_not_append_personal_examples() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(cache_dir.join("pages.en/linux")).unwrap();
    fs::create_dir_all(cache_dir.join("taps/personal/pages/common")).unwrap();
    fs::write(
        cache_dir.join("pages.en/linux/foo.md"),
        "# foo\n\n> Test page.\n\n- Official example:\n\n`foo --official`\n",
    )
    .unwrap();
    fs::write(
        cache_dir.join("taps/personal/pages/common/foo.md"),
        "# foo\n\n> Personal page.\n\n- Personal example:\n\n`foo --personal`\n",
    )
    .unwrap();

    let cfg_path = tmp.path().join("config.toml");
    make_cfg(
        &cfg_path,
        &cache_dir,
        r#"[[taps]]
name = "personal"
url = "https://example.com/personal.git"
enabled = true"#,
    );

    let mut cmd = Command::cargo_bin("tldr").unwrap();
    cmd.args([
        "--config",
        cfg_path.to_str().unwrap(),
        "--platform",
        "linux",
        "--raw",
        "foo",
    ]);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("# foo"));
    assert!(!stdout.contains("Personal additions"));
    assert!(!stdout.contains("foo --personal"));
}

#[test]
fn tap_list_prints_empty_state() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");

    let output = Command::cargo_bin("tldr")
        .unwrap()
        .args(["--config", cfg_path.to_str().unwrap(), "--tap-list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("no taps configured"));
}

#[test]
fn tap_add_list_remove_lifecycle() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");
    let remote = init_remote_with_page(
        tmp.path(),
        "# foo\n\n> Remote page.\n\n- Example:\n\n`foo --from-remote`\n",
    );

    let add = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-add",
            "personal",
            &remote,
        ])
        .output()
        .unwrap();
    assert!(add.status.success());
    assert!(cache_dir.join("taps/personal/.git").is_dir());

    let list = Command::cargo_bin("tldr")
        .unwrap()
        .args(["--config", cfg_path.to_str().unwrap(), "--tap-list"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let listed = String::from_utf8(list.stdout).unwrap();
    assert!(listed.contains("personal ->"));

    let dup_add = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-add",
            "personal",
            &remote,
        ])
        .output()
        .unwrap();
    assert!(!dup_add.status.success());

    let remove = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-remove",
            "personal",
        ])
        .output()
        .unwrap();
    assert!(remove.status.success());
    assert!(!cache_dir.join("taps/personal").exists());
}

#[test]
fn tap_update_all_pulls_new_commit() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");
    let remote = init_remote_with_page(
        tmp.path(),
        "# foo\n\n> Remote page.\n\n- First:\n\n`foo --first`\n",
    );

    let add = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-add",
            "personal",
            &remote,
        ])
        .output()
        .unwrap();
    assert!(add.status.success());

    // Push a second commit to the remote.
    let work = tmp.path().join("work");
    run_git(tmp.path(), &["clone", &remote, work.to_str().unwrap()]);
    run_git(
        &work,
        &["config", "user.email", "tldr-tests@example.invalid"],
    );
    run_git(&work, &["config", "user.name", "tlrc tests"]);
    fs::write(
        work.join("pages/common/foo.md"),
        "# foo\n\n> Remote page.\n\n- First:\n\n`foo --first`\n\n- Second:\n\n`foo --second`\n",
    )
    .unwrap();
    run_git(&work, &["add", "."]);
    run_git(&work, &["commit", "-m", "second"]);
    run_git(&work, &["push"]);

    let upd = Command::cargo_bin("tldr")
        .unwrap()
        .args(["--config", cfg_path.to_str().unwrap(), "--tap-update-all"])
        .output()
        .unwrap();
    assert!(upd.status.success());

    // Verify updated personal example shows up in rendered output.
    fs::create_dir_all(cache_dir.join("pages.en/linux")).unwrap();
    fs::write(
        cache_dir.join("pages.en/linux/foo.md"),
        "# foo\n\n> Official page.\n\n- Official:\n\n`foo --official`\n",
    )
    .unwrap();
    let output = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--platform",
            "linux",
            "foo",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("foo --second"));
}

#[test]
fn tap_update_single_pulls_new_commit() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");
    let remote = init_remote_with_page(
        tmp.path(),
        "# foo\n\n> Remote page.\n\n- First:\n\n`foo --first`\n",
    );

    let add = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-add",
            "personal",
            &remote,
        ])
        .output()
        .unwrap();
    assert!(add.status.success());

    let work = tmp.path().join("work-one");
    run_git(tmp.path(), &["clone", &remote, work.to_str().unwrap()]);
    run_git(
        &work,
        &["config", "user.email", "tldr-tests@example.invalid"],
    );
    run_git(&work, &["config", "user.name", "tlrc tests"]);
    fs::write(
        work.join("pages/common/foo.md"),
        "# foo\n\n> Remote page.\n\n- First:\n\n`foo --first`\n\n- Updated:\n\n`foo --updated`\n",
    )
    .unwrap();
    run_git(&work, &["add", "."]);
    run_git(&work, &["commit", "-m", "update"]);
    run_git(&work, &["push"]);

    let upd = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-update",
            "personal",
        ])
        .output()
        .unwrap();
    assert!(upd.status.success());

    fs::create_dir_all(cache_dir.join("pages.en/linux")).unwrap();
    fs::write(
        cache_dir.join("pages.en/linux/foo.md"),
        "# foo\n\n> Official page.\n\n- Official:\n\n`foo --official`\n",
    )
    .unwrap();
    let output = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--platform",
            "linux",
            "foo",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("foo --updated"));
}

#[test]
fn tap_update_unknown_fails() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");

    let output = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-update",
            "missing",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn tap_remove_unknown_fails() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");

    let output = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-remove",
            "missing",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn tap_add_invalid_remote_does_not_persist() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let cfg_path = tmp.path().join("config.toml");
    make_cfg(&cfg_path, &cache_dir, "");
    let invalid_remote = tmp.path().join("does-not-exist.git");

    let add = Command::cargo_bin("tldr")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--tap-add",
            "personal",
            invalid_remote.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!add.status.success());

    let cfg = fs::read_to_string(&cfg_path).unwrap();
    assert!(!cfg.contains("personal"));
}
