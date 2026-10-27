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
fn heading_punctuation_is_fixed_in_stdout_check_and_write_modes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".markdownlint.json"),
        r#"{"default":false,"MD026":true}"#,
    )
    .unwrap();
    fs::write(root.join("doc.md"), "# Hello!\r\n").unwrap();
    let output = run(root, &["-"], "# Hello!\r\n");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"# Hello\r\n");
    assert!(output.stderr.is_empty());
    let output = run(root, &["--check", "doc.md"], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(root.join("doc.md")).unwrap(), b"# Hello!\r\n");
    let output = run(root, &["--write", "doc.md"], "");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fs::read(root.join("doc.md")).unwrap(), b"# Hello\r\n");
    assert_eq!(run(root, &["--check", "doc.md"], "").status.code(), Some(0));
}

#[test]
fn disabled_heading_fixes_still_report_errors_or_warnings_in_every_mode() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (severity, status) in [("error", 1), ("warning", 0)] {
        fs::write(root.join(".markdownlint-cli2.jsonc"), format!(r#"{{"fix":false,"config":{{"default":false,"MD026":{{"severity":"{severity}"}}}}}}"#)).unwrap();
        fs::write(root.join("doc.md"), "# Hello!\n").unwrap();
        for args in [
            vec!["-"],
            vec!["--check", "doc.md"],
            vec!["--write", "doc.md"],
        ] {
            let output = run(root, &args, "# Hello!\n");
            assert_eq!(output.status.code(), Some(status));
            assert!(String::from_utf8_lossy(&output.stderr).contains("1:8:"));
            assert!(String::from_utf8_lossy(&output.stderr).contains("MD026"));
            assert_eq!(fs::read(root.join("doc.md")).unwrap(), b"# Hello!\n");
        }
    }
}

#[test]
fn heading_findings_use_positions_after_multiline_reflow() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".markdownlint.json"),
        r#"{"default":false,"MD013":{"line_length":10},"MD026":true}"#,
    )
    .unwrap();
    let output = run(root, &["-"], "alpha\nbeta\n\n# 😀 title!\n");
    assert_eq!(output.stdout, "alpha beta\n\n# 😀 title\n".as_bytes());
    // MD026 runs before final MD013 diagnostics, so heading punctuation no longer
    // contributes to the reported width or excess span.
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
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
fn reflow_uses_discovered_configuration_in_stdout_check_and_write_modes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".markdownlint.json"),
        r#"{"MD013":{"line_length":10}}"#,
    )
    .unwrap();
    fs::write(root.join("doc.md"), "alpha beta gamma\r\n").unwrap();
    let output = run(root, &["-"], "alpha beta gamma\r\n");
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert_eq!(output.stdout, b"alpha beta\r\ngamma\r\n");
    assert!(output.stderr.is_empty());
    let output = run(root, &["--check", "doc.md"], "");
    assert_eq!(output.status.code(), Some(1), "{:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(root.join("doc.md")).unwrap(),
        b"alpha beta gamma\r\n"
    );
    let output = run(root, &["--write", "doc.md"], "");
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(root.join("doc.md")).unwrap(),
        b"alpha beta\r\ngamma\r\n"
    );
    let output = run(root, &["--check", "doc.md"], "");
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stdout.is_empty());
}

#[test]
fn cli2_fix_false_prevents_reflow_in_every_mode() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"MD013":{"line_length":10}}}"#,
    )
    .unwrap();
    fs::write(root.join("doc.md"), "alpha beta gamma").unwrap();
    let output = run(root, &["doc.md"], "");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"alpha beta gamma");
    assert!(String::from_utf8_lossy(&output.stderr).contains("MD013"));
    for mode in ["--check", "--write"] {
        let output = run(root, &[mode, "doc.md"], "");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(root.join("doc.md")).unwrap(), b"alpha beta gamma");
    }
}

#[test]
fn diagnostic_columns_ignore_a_leading_bom() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"MD013":{"line_length":10,"strict":true}}}"#,
    )
    .unwrap();
    let output = run(dir.path(), &["-"], "\u{feff}alpha beta gamma");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("-:1:11: Error MD013:"));
}

#[test]
fn remaining_error_and_warning_findings_control_status_without_losing_output() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (severity, expected_status) in [("error", 1), ("warning", 0)] {
        fs::write(root.join(".markdownlint.json"), format!(r#"{{"default":false,"MD013":{{"line_length":10,"strict":true,"severity":"{severity}"}}}}"#)).unwrap();
        let text = "alpha verylongtoken tail\n";
        let expected = "alpha\nverylongtoken\ntail\n";
        let output = run(root, &["-"], text);
        assert_eq!(output.status.code(), Some(expected_status));
        assert_eq!(output.stdout, expected.as_bytes());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("-:2:11:"));
        assert!(stderr.contains("Expected: 10; Actual: 13"));
        fs::write(root.join("doc.md"), text).unwrap();
        let output = run(root, &["--write", "doc.md"], "");
        assert_eq!(output.status.code(), Some(expected_status));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read_to_string(root.join("doc.md")).unwrap(), expected);
        let output = run(root, &["--check", "doc.md"], "");
        assert_eq!(output.status.code(), Some(expected_status));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn help_describes_md013_without_claiming_all_rule_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(dir.path(), &["--help"], "");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("MD013"));
    assert!(!help.contains("no rule fixes yet"));
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
