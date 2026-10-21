use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_preserves_bytes_and_check_emits_no_content() {
    let text = "\u{feff}# Hello!\r\n\r\nDo not reflow yet.  \r\n".as_bytes();
    let output = run(&["-"], text);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, text);
    assert!(output.stderr.is_empty());
    let output = run(&["--check", "-"], text);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn rejects_ambiguous_modes_and_unsupported_arguments() {
    for args in [
        vec![],
        vec!["--write", "-"],
        vec!["--write", "--check", "a"],
        vec!["-", "-"],
        vec!["-", "file"],
        vec!["a", "b"],
        vec!["--config", "config.json", "a"],
    ] {
        let output = run(&args, b"");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty());
    }
    for flag in ["--help", "--version"] {
        let output = run(&[flag], b"");
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn files_are_unchanged_and_stdout_is_document_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.md");
    std::fs::write(&path, b"# Hello!\r\nno newline").unwrap();
    let path = path.to_str().unwrap();
    let output = run(&[path], b"");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"# Hello!\r\nno newline");
    assert!(output.stderr.is_empty());
    for mode in ["--write", "--check"] {
        let output = run(&[mode, path, path], b"");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert_eq!(std::fs::read(path).unwrap(), b"# Hello!\r\nno newline");
    }
}

#[test]
fn invalid_utf8_and_missing_files_are_operational_failures() {
    let output = run(&["-"], &[0xff]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("missing-a");
    let b = dir.path().join("missing-b");
    let output = run(&["--check", a.to_str().unwrap(), b.to_str().unwrap()], b"");
    assert_eq!(output.status.code(), Some(2));
    let errors = String::from_utf8(output.stderr).unwrap();
    assert!(errors.contains("missing-a") && errors.contains("missing-b"));
}

#[test]
fn double_dash_accepts_literal_dash_prefixed_filename() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("-literal.md"), "original").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--", "-literal.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"original");
}

#[cfg(target_os = "linux")]
#[test]
fn stderr_failure_returns_operational_exit_instead_of_panicking() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .args(["a", "b"])
        .stderr(full)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn write_rejects_symlink_even_after_a_duplicate_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("real.md");
    let alias = dir.path().join("alias.md");
    std::fs::write(&path, "original").unwrap();
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    let output = run(
        &["--write", path.to_str().unwrap(), alias.to_str().unwrap()],
        b"",
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
}
