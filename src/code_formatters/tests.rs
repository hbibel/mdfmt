use super::{
    adapters::Adapter,
    format_stable,
    runner::{RunLimits, run},
    stable_output,
};
use std::{
    path::PathBuf,
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn fixture(mode: &str) -> Command {
    static BUILD: OnceLock<tempfile::TempDir> = OnceLock::new();
    let directory = BUILD.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let status = Command::new("rustc")
            .args(["--edition=2024", "-O"])
            .arg(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/support/code_formatter_fixture.rs"),
            )
            .arg("-o")
            .arg(
                directory
                    .path()
                    .join(format!("fixture{}", std::env::consts::EXE_SUFFIX)),
            )
            .status()
            .unwrap();
        assert!(status.success());
        directory
    });
    let mut command = Command::new(
        directory
            .path()
            .join(format!("fixture{}", std::env::consts::EXE_SUFFIX)),
    );
    command.arg(mode);
    command
}

fn limits() -> RunLimits {
    RunLimits {
        timeout: Duration::from_millis(200),
        stdout: 128 * 1024,
        stderr: 128 * 1024,
    }
}

#[test]
fn runner_preserves_partial_failure_and_bounds_both_pipes() {
    let limits = RunLimits {
        timeout: Duration::from_secs(3),
        ..limits()
    };
    assert_eq!(
        run(fixture("echo"), b"code\r\n", &limits).unwrap(),
        b"code\r\n"
    );
    assert_eq!(
        run(fixture("partial"), b"original", &limits)
            .unwrap_err()
            .code(),
        "formatter-failed"
    );
    let output = run(fixture("both"), b"", &limits).unwrap();
    assert_eq!(output, vec![b'o'; 64 * 1024]);
    let error = run(
        fixture("both"),
        b"",
        &RunLimits {
            stderr: 1024,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), "formatter-output-limit");
    let error = run(fixture("flood"), b"", &limits).unwrap_err();
    assert_eq!(error.code(), "formatter-output-limit");
    let error = run(
        Command::new("mdfmt-no-such-formatter-8def8494"),
        b"",
        &limits,
    )
    .unwrap_err();
    assert_eq!(error.code(), "formatter-unavailable");
    assert!(error.message().contains("mdfmt-no-such-formatter-8def8494"));
}

#[test]
fn runner_deadline_covers_blocked_stdin_and_descendant_held_output() {
    let start = Instant::now();
    let error = run(fixture("sleep"), &vec![b'x'; 1024 * 1024], &limits()).unwrap_err();
    assert_eq!(error.code(), "formatter-timeout");
    assert!(start.elapsed() < Duration::from_secs(2));
    let start = Instant::now();
    let result = run(fixture("descendant"), b"", &limits());
    assert!(start.elapsed() < Duration::from_secs(2));
    match result {
        Ok(output) => assert!(output.is_empty()),
        Err(error) => assert_eq!(error.code(), "formatter-timeout"),
    }
}

#[test]
fn stable_format_rejects_invalid_empty_and_alternating_results() {
    for mode in ["invalid", "empty", "nul"] {
        let error =
            stable_output("original\n", |input| run(fixture(mode), input, &limits())).unwrap_err();
        assert_eq!(error.code(), "formatter-failed");
    }
    let directory = tempfile::tempdir().unwrap();
    let counter = directory.path().join("counter");
    let error = stable_output("original\n", |input| {
        let mut command = fixture("alternating");
        command.arg(&counter);
        run(command, input, &limits())
    })
    .unwrap_err();
    assert_eq!(error.code(), "formatter-unstable");
    assert_eq!(std::fs::read_to_string(counter).unwrap(), "2");
    let output = stable_output("code\r\n", |input| {
        run(fixture("uppercase"), input, &limits())
    })
    .unwrap();
    assert_eq!(output, "CODE\n");
}

#[test]
fn adapter_commands_are_isolated_and_width_specific() {
    let directory = tempfile::tempdir().unwrap();
    for (language, program, args) in [
        (
            "rust",
            "rustfmt",
            vec!["--edition", "2024", "--emit", "stdout", "--config-path"],
        ),
        (
            "rs",
            "rustfmt",
            vec!["--edition", "2024", "--emit", "stdout", "--config-path"],
        ),
        ("go", "gofmt", vec![]),
        ("golang", "gofmt", vec![]),
        (
            "py",
            "black",
            vec!["--isolated", "--quiet", "--line-length", "38", "-"],
        ),
        (
            "python",
            "black",
            vec!["--isolated", "--quiet", "--line-length", "38", "-"],
        ),
        (
            "json",
            "prettier",
            vec![
                "--no-config",
                "--no-editorconfig",
                "--parser",
                "json",
                "--print-width",
                "38",
            ],
        ),
    ] {
        let adapter = Adapter::for_language(language).unwrap();
        let command = adapter.command(38, directory.path()).unwrap();
        assert_eq!(command.get_program(), program);
        assert_eq!(command.get_current_dir(), Some(directory.path()));
        let actual: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_str().unwrap())
            .collect();
        if program == "rustfmt" {
            assert_eq!(&actual[..5], args);
            assert_eq!(actual.len(), 6);
            let path = PathBuf::from(actual[5]);
            assert_eq!(path.parent(), Some(directory.path()));
            assert_eq!(std::fs::read_to_string(path).unwrap(), "max_width = 38\n");
        } else {
            assert_eq!(actual, args);
        }
    }
    for language in [
        "",
        "text",
        "javascript",
        "json extra",
        "rust,ignore",
        "Rust",
    ] {
        assert!(Adapter::for_language(language).is_none());
    }
    for width in [0, 4097, usize::MAX] {
        assert_eq!(
            Adapter::for_language("rust")
                .unwrap()
                .command(width, directory.path())
                .unwrap_err()
                .code(),
            "formatter-skipped"
        );
    }
}

#[test]
fn installed_rustfmt_and_gofmt_have_real_stable_smoke_coverage_when_available() {
    for (language, input, expected) in [
        (
            "rust",
            "fn main(){println!(\"hello\");}\n",
            "fn main() {\n    println!(\"hello\");\n}\n",
        ),
        (
            "go",
            "package main\nfunc main(){println(\"hello\")}\n",
            "package main\n\nfunc main() { println(\"hello\") }\n",
        ),
    ] {
        let result = format_stable(
            Adapter::for_language(language).unwrap(),
            input,
            40,
            Instant::now() + Duration::from_secs(15),
        );
        match result {
            Ok(output) => assert_eq!(output, expected),
            Err(error) if error.code() == "formatter-unavailable" => {
                eprintln!("SKIP actual {language} formatter: unavailable")
            }
            Err(error) => panic!("actual {language} formatter: {error:?}"),
        }
    }
}
