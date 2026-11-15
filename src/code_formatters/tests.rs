use super::{
    adapters::Adapter,
    runner::{RunLimits, run},
};
#[cfg(unix)]
use super::{format_stable, stable_output};
use std::{path::PathBuf, process::Command, time::Duration};
#[cfg(unix)]
use std::{sync::OnceLock, time::Instant};

#[cfg(unix)]
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn cleanup_failure_has_a_bounded_return() {
    if std::env::var_os("MDFMT_TEST_CLEANUP_CHILD").is_some() {
        #[repr(C)]
        struct Filter {
            code: u16,
            jt: u8,
            jf: u8,
            k: u32,
        }
        #[repr(C)]
        struct Program {
            len: u16,
            filter: *const Filter,
        }
        unsafe extern "C" {
            fn prctl(option: i32, ...) -> i32;
        }
        let filters = [
            Filter {
                code: 0x20,
                jt: 0,
                jf: 0,
                k: 0,
            },
            Filter {
                code: 0x15,
                jt: 0,
                jf: 3,
                k: 62,
            },
            Filter {
                code: 0x20,
                jt: 0,
                jf: 0,
                k: 24,
            },
            Filter {
                code: 0x15,
                jt: 0,
                jf: 1,
                k: 9,
            },
            Filter {
                code: 0x06,
                jt: 0,
                jf: 0,
                k: 0x00050001,
            },
            Filter {
                code: 0x06,
                jt: 0,
                jf: 0,
                k: 0x7fff0000,
            },
        ];
        let program = Program {
            len: filters.len() as u16,
            filter: filters.as_ptr(),
        };
        unsafe {
            assert_eq!(prctl(38, 1, 0, 0, 0), 0);
            assert_eq!(prctl(22, 2, &program), 0);
        }
        let mut command = Command::new(std::env::var_os("MDFMT_TEST_CLEANUP_FIXTURE").unwrap());
        command.arg("shortsleep");
        let start = Instant::now();
        let error = run(
            command,
            b"",
            &RunLimits {
                timeout: Duration::from_millis(100),
                ..limits()
            },
        )
        .unwrap_err();
        assert!(
            start.elapsed() < Duration::from_millis(600),
            "cleanup exceeded deadline: {error:?}"
        );
        assert_eq!(error.code(), "formatter-cleanup-failed");
        assert!(error.message().contains("Operation not permitted"));
        let pid = std::env::var_os("MDFMT_TEST_HELPER_PID").unwrap();
        let original_pid = std::fs::read_to_string(&pid).unwrap();
        let settings = crate::config::ResolvedConfig {
            rules: crate::config::RuleConfig::from_value(
                serde_json::json!({"default":false,"MD013":true}),
            )
            .unwrap(),
            ..Default::default()
        };
        let input = "```json\n{}\n```\n\n```json\n{}\n```\n";
        let result = super::format_with_code_formatters(
            input,
            &crate::FormatOptions::default(),
            &settings,
            &super::CodeFormatOptions { enabled: true },
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|finding| finding.code == "formatter-cleanup-failed")
                .count(),
            1
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|finding| finding.code == "formatter-skipped")
                .count(),
            1
        );
        assert_eq!(std::fs::read_to_string(pid).unwrap(), original_pid);
        return;
    }
    let executable = fixture("shortsleep").get_program().to_os_string();
    let directory = tempfile::tempdir().unwrap();
    let pid = directory.path().join("helper.pid");
    let log = directory.path().join("result.log");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "code_formatters::tests::cleanup_failure_has_a_bounded_return",
            "--nocapture",
        ])
        .env("MDFMT_TEST_CLEANUP_CHILD", "1")
        .env("MDFMT_TEST_CLEANUP_FIXTURE", executable)
        .env("MDFMT_TEST_HELPER_PID", &pid)
        .stdout(std::fs::File::create(&log).unwrap())
        .stderr(std::fs::File::options().append(true).open(&log).unwrap())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(1) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if let Ok(pid) = std::fs::read_to_string(pid) {
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", pid.trim())])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    assert!(
        status.is_some_and(|status| status.success()),
        "bounded cleanup regression failed: {}",
        std::fs::read_to_string(log).unwrap()
    );
}

#[test]
#[cfg(unix)]
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
#[cfg(unix)]
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
#[cfg(unix)]
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
                "--no-plugin-search",
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
#[cfg(unix)]
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

#[cfg(not(unix))]
#[test]
fn unsupported_transport_fails_closed_before_process_execution() {
    let error = run(Command::new("mdfmt-do-not-execute"), b"code", &limits()).unwrap_err();
    assert_eq!(error.code(), "formatter-skipped");
    assert!(error.message().contains("not available on this platform"));
}
