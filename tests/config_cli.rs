use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(root: &Path, args: &[&str], text: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn explicit_configuration_flags_and_stdin_are_supported() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("config.toml"), "[nested]\nMD013 = false\n").unwrap();
    let output = run(
        root,
        &[
            "--config",
            "config.toml",
            "--config-pointer",
            "/nested",
            "-",
        ],
        "unchanged\r\n",
    );
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert_eq!(output.stdout, b"unchanged\r\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn config_errors_and_invalid_inline_configuration_prevent_output_and_writes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("doc.md"), "original\n").unwrap();
    fs::write(root.join(".markdownlint.json"), "{\"MD999\":true}").unwrap();
    for args in [vec!["doc.md"], vec!["--write", "doc.md"]] {
        let output = run(root, &args, "");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid-config"));
        assert_eq!(fs::read(root.join("doc.md")).unwrap(), b"original\n");
    }
    fs::write(root.join(".markdownlint.json"), "{}").unwrap();
    let output = run(root, &["-"], "<!-- markdownlint-configure-file {bad -->");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn globs_show_found_and_no_progress_keep_stdout_clean() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("a.md"), "a").unwrap();
    fs::write(root.join("b.md"), "b").unwrap();
    fs::write(
        root.join(".markdownlint-cli2.jsonc"),
        r#"{"globs":["*.md"],"showFound":true,"fix":false}"#,
    )
    .unwrap();
    let output = run(root, &["--check"], "");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("a.md"));
    let output = run(root, &["--check", "--no-globs"], "");
    assert_eq!(output.status.code(), Some(2));
    fs::write(
        root.join(".markdownlint-cli2.jsonc"),
        r#"{"globs":["*.md"],"showFound":true,"noProgress":true}"#,
    )
    .unwrap();
    let output = run(root, &["--check"], "");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(output.stdout.is_empty());
}

#[test]
fn no_inline_config_bypasses_invalid_directives_and_double_dash_is_literal() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".markdownlint-cli2.jsonc"),
        r#"{"noInlineConfig":true}"#,
    )
    .unwrap();
    fs::write(
        root.join("-literal*.md"),
        "<!-- markdownlint-configure-file {bad -->",
    )
    .unwrap();
    let output = run(root, &["--", "-literal*.md"], "");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"<!-- markdownlint-configure-file {bad -->");
}

#[test]
fn home_relative_extends_uses_the_referring_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("base.json"), r#"{"default":false}"#).unwrap();
    fs::write(
        root.join(".markdownlint.json"),
        r#"{"extends":"~/base.json"}"#,
    )
    .unwrap();
    fs::write(root.join("doc.md"), "unchanged").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(root)
        .env("HOME", root)
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert_eq!(output.stdout, b"unchanged");
}
