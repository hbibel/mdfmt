use mdfmt::{
    CodeFormatOptions, FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_code_formatters,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

const RAW: &str = "{\"one\": 1, \"two\": 2, \"three\": 3}";
const PRETTY: &str = "{\n  \"one\": 1,\n  \"two\": 2,\n  \"three\": 3\n}\n";

fn fixture() -> &'static Path {
    static FIXTURE: OnceLock<tempfile::TempDir> = OnceLock::new();
    let directory = FIXTURE.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let status = Command::new("rustc")
            .args(["--edition=2024", "-O"])
            .arg(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/support/code_formatter_fixture.rs"),
            )
            .arg("-o")
            .arg(
                dir.path()
                    .join(format!("fixture{}", std::env::consts::EXE_SUFFIX)),
            )
            .status()
            .unwrap();
        assert!(status.success());
        dir
    });
    directory.path()
}
fn setup(rules: Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("bin")).unwrap();
    for name in ["rustfmt", "gofmt", "black", "prettier"] {
        fs::copy(
            fixture().join(format!("fixture{}", std::env::consts::EXE_SUFFIX)),
            dir.path()
                .join("bin")
                .join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
    }
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":rules}).to_string(),
    )
    .unwrap();
    dir
}
fn settings() -> Value {
    json!({"default":false,"MD013":{"line_length":80,"code_block_line_length":20,"strict":true}})
}
fn run(dir: &Path, args: &[&str], mode: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir)
        .args(args)
        .env("PATH", dir.join("bin"))
        .env("MDFMT_FIXTURE_MODE", mode)
        .env("MDFMT_FIXTURE_LOG", dir.join("calls.log"))
        .output()
        .unwrap()
}
fn calls(dir: &Path) -> String {
    fs::read_to_string(dir.join("calls.log")).unwrap_or_default()
}

#[test]
fn successful_formatter_restores_quote_and_list_prefixes() {
    for (opening, prefix, closing) in [
        ("```json\n", "", "```\n"),
        ("> ```json\n", "> ", "> ```\n"),
        ("- ```json\n", "  ", "  ```\n"),
        ("> - ```json\n", ">   ", ">   ```\n"),
    ] {
        for newline in ["\n", "\r\n", "\r"] {
            let input = format!("{opening}{prefix}{RAW}\n{closing}").replace('\n', newline);
            let expected = format!(
                "{opening}{}{closing}",
                PRETTY
                    .lines()
                    .map(|line| format!("{prefix}{line}\n"))
                    .collect::<String>()
            )
            .replace('\n', newline);
            let (input, expected) = if newline == "\r\n" {
                (format!("\u{feff}{input}"), format!("\u{feff}{expected}"))
            } else {
                (input, expected)
            };
            let dir = setup(settings());
            fs::write(dir.path().join("doc.md"), &input).unwrap();
            let first = run(dir.path(), &["--format-code", "--write", "doc.md"], "json");
            assert_eq!(
                first.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&first.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                expected
            );
            assert_eq!(calls(dir.path()).matches("CALL ").count(), 2);
            assert!(
                calls(dir.path())
                    .contains(&format!("\"--print-width\", \"{}\"", 20 - prefix.len()))
            );
            assert!(calls(dir.path()).contains(&format!("INPUT {:?}", format!("{RAW}\n"))));
            let second = run(dir.path(), &["--format-code", "--check", "doc.md"], "json");
            assert_eq!(
                second.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&second.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                expected
            );
        }
    }
}

#[test]
fn disabled_or_unowned_fences_never_spawn() {
    let ordinary = format!("```json\n{RAW}\n```\n");
    for (input, rules, fix) in [
        (
            ordinary.clone(),
            json!({"default":false,"MD013":false}),
            true,
        ),
        (
            ordinary.clone(),
            json!({"default":false,"MD013":{"code_blocks":false}}),
            true,
        ),
        (ordinary.clone(), settings(), false),
        (
            format!("<!-- markdownlint-disable-next-line MD013 -->\n{ordinary}"),
            settings(),
            true,
        ),
        (
            format!(
                "<!-- markdownlint-disable MD013 -->\n{ordinary}<!-- markdownlint-enable MD013 -->\n"
            ),
            settings(),
            true,
        ),
        (format!("```json extra\n{RAW}\n```\n"), settings(), true),
        (
            "```python\nnote = \"<!-- markdownlint-disable MD013 -->\"\na=1\n```\n".into(),
            settings(),
            true,
        ),
        (format!("```json\n{RAW}\n"), settings(), true),
        (format!("```\n{RAW}\n```\n"), settings(), true),
        (format!("```unknown\n{RAW}\n```\n"), settings(), true),
        (
            format!("| code |\n| - |\n| ```json |\n| {RAW} |\n| ``` |\n"),
            settings(),
            true,
        ),
        (format!("    {RAW}\n"), settings(), true),
        (
            format!("<div>\n\n```json\n{RAW}\n```\n\n</div>\n"),
            settings(),
            true,
        ),
        (format!(">\t```json\n>\t{RAW}\n>\t```\n"), settings(), true),
    ] {
        let dir = setup(rules.clone());
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"fix":fix,"config":rules}).to_string(),
        )
        .unwrap();
        fs::write(dir.path().join("doc.md"), &input).unwrap();
        let result = run(dir.path(), &["--format-code", "--write", "doc.md"], "json");
        assert_ne!(
            result.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("doc.md")).unwrap(),
            input
        );
        assert!(calls(dir.path()).is_empty(), "{input:?}");
    }
    let dir = setup(settings());
    fs::write(dir.path().join("doc.md"), &ordinary).unwrap();
    let result = run(dir.path(), &["--write", "doc.md"], "json");
    assert_eq!(result.status.code(), Some(1));
    assert!(calls(dir.path()).is_empty());
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        ordinary
    );
}

#[test]
fn failures_preserve_original_bodies_and_final_findings() {
    let input = format!("```json\n{RAW}\n```\n");
    for (mode, code) in [
        ("partial", "formatter-failed"),
        ("invalid", "formatter-failed"),
        ("empty", "formatter-failed"),
        ("nul", "formatter-failed"),
        ("unstable", "formatter-unstable"),
        ("flood", "formatter-output-limit"),
        ("unsafe", "formatter-unsafe"),
        ("sleep", "formatter-timeout"),
    ] {
        let dir = setup(settings());
        fs::write(dir.path().join("doc.md"), &input).unwrap();
        let result = run(dir.path(), &["--format-code", "--write", "doc.md"], mode);
        assert_eq!(
            result.status.code(),
            Some(1),
            "{mode}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("doc.md")).unwrap(),
            input
        );
        let errors = String::from_utf8_lossy(&result.stderr);
        assert!(errors.contains(&format!("Warning {code}:")), "{errors}");
        assert!(errors.contains("Error MD013:"));
        assert!(!errors.contains("partial invalid output"));
    }
    let dir = setup(settings());
    fs::remove_file(
        dir.path()
            .join("bin")
            .join(format!("prettier{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    fs::write(dir.path().join("doc.md"), &input).unwrap();
    let result = run(dir.path(), &["--format-code", "--write", "doc.md"], "json");
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("formatter-unavailable"));
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        input
    );
}

#[test]
fn independent_successes_and_final_warning_anchors_survive_prose_reflow() {
    let input = format!(
        "A paragraph with enough ordinary words to wrap before the failed fence.\n\n```json\n{{\"bad\": true}}\n```\n\n```json\n{RAW}\n```\n"
    );
    let expected = format!(
        "A paragraph with enough\nordinary words to wrap\nbefore the failed fence.\n\n```json\n{{\"bad\": true}}\n```\n\n```json\n{PRETTY}```\n"
    );
    let dir = setup(
        json!({"default":false,"line-length":{"line_length":28,"code_block_line_length":20,"strict":true,"severity":"warning"}}),
    );
    fs::write(dir.path().join("doc.md"), &input).unwrap();
    for _ in 0..2 {
        let result = run(
            dir.path(),
            &["--format-code", "--write", "doc.md"],
            "dispatch",
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(
            fs::read_to_string(dir.path().join("doc.md")).unwrap(),
            expected
        );
        let errors = String::from_utf8_lossy(&result.stderr);
        assert!(
            errors.contains("doc.md:5:1: Warning formatter-failed:"),
            "{errors}"
        );
        assert_eq!(errors.matches("formatter-failed:").count(), 1);
    }
}

#[test]
fn code_comments_stay_literal_and_cli_check_does_not_write() {
    let input = "```python\n# markdownlint-disable MD013\na=1\n```\n";
    let expected = "```python\n# MARKDOWNLINT-DISABLE MD013\nA=1\n```\n";
    let dir = setup(json!({"default":false,"MD013":{"line_length":80,"strict":true}}));
    fs::write(dir.path().join("doc.md"), input).unwrap();
    let check = run(
        dir.path(),
        &["--format-code", "--check", "doc.md"],
        "uppercase",
    );
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        input
    );
    let stdout = run(dir.path(), &["--format-code", "doc.md"], "uppercase");
    assert_eq!(stdout.status.code(), Some(0));
    assert_eq!(String::from_utf8(stdout.stdout).unwrap(), expected);
    assert_eq!(calls(dir.path()).matches("CALL ").count(), 4);
}

#[test]
fn explicit_library_api_reconstructs_original_edits_and_preserves_pure_controls() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD013":{"line_length":32,"strict":true}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "A paragraph with enough words to wrap cleanly before code.\n\n```rust\nfn main(){println!(\"hello\");}\n```\n";
    let options = CodeFormatOptions { enabled: true };
    let result =
        format_with_code_formatters(input, &FormatOptions::default(), &settings, &options).unwrap();
    if result
        .diagnostics
        .iter()
        .any(|d| d.code == "formatter-unavailable")
    {
        eprintln!("SKIP actual rustfmt API smoke: unavailable");
        return;
    }
    assert!(
        result
            .output
            .contains("fn main() {\n    println!(\"hello\");\n}\n"),
        "{}",
        result.output
    );
    let mut reconstructed = String::new();
    let mut cursor = 0;
    for edit in &result.edits {
        reconstructed.push_str(&input[cursor..edit.span.start]);
        reconstructed.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    reconstructed.push_str(&input[cursor..]);
    assert_eq!(reconstructed, result.output);
    let again = format_with_code_formatters(
        &result.output,
        &FormatOptions::default(),
        &settings,
        &options,
    )
    .unwrap();
    assert_eq!(again.output, result.output);
    assert!(again.edits.is_empty());
    for (format, settings, code) in [
        (FormatOptions { fix: false }, settings.clone(), options),
        (
            FormatOptions::default(),
            ResolvedConfig {
                fix: false,
                ..settings.clone()
            },
            options,
        ),
    ] {
        let result = format_with_code_formatters(input, &format, &settings, &code).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| !d.code.starts_with("formatter-"))
        );
    }
    let disabled = format_with_code_formatters(
        input,
        &FormatOptions::default(),
        &settings,
        &CodeFormatOptions::default(),
    )
    .unwrap();
    assert!(!disabled.output.contains("fn main() {"));
    assert!(
        disabled
            .diagnostics
            .iter()
            .all(|d| !d.code.starts_with("formatter-"))
    );
}

#[test]
fn preceding_success_reanchors_later_failures_and_preserves_physical_failure_bytes() {
    for newline in ["\n", "\r\n", "\r"] {
        let dir = setup(settings());
        let input = format!("```json\n{RAW}\n```\n\n```json\n{{\"bad\": true}}\n```\n")
            .replace('\n', newline);
        let expected = format!("```json\n{PRETTY}```\n\n```json\n{{\"bad\": true}}\n```\n")
            .replace('\n', newline);
        let (input, expected) = if newline == "\r\n" {
            (format!("\u{feff}{input}"), format!("\u{feff}{expected}"))
        } else {
            (input, expected)
        };
        fs::write(dir.path().join("doc.md"), &input).unwrap();
        for _ in 0..2 {
            let result = run(
                dir.path(),
                &["--format-code", "--write", "doc.md"],
                "dispatch",
            );
            assert_eq!(
                result.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                expected
            );
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("doc.md:9:1: Warning formatter-failed:")
            );
        }
    }
}

#[test]
fn body_count_and_width_limits_skip_without_unbounded_process_execution() {
    let dir = setup(json!({"default":false,"MD013":{"line_length":80,"strict":true}}));
    let input = format!("```json\n{RAW}\n```\n\n").repeat(65);
    let mut expected = format!("```json\n{PRETTY}```\n\n").repeat(64);
    expected.push_str(&format!("```json\n{RAW}\n```\n\n"));
    fs::write(dir.path().join("doc.md"), input).unwrap();
    let result = run(dir.path(), &["--format-code", "--write", "doc.md"], "json");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        expected
    );
    assert_eq!(calls(dir.path()).matches("CALL ").count(), 128);
    assert_eq!(
        String::from_utf8_lossy(&result.stderr)
            .matches("formatter-skipped:")
            .count(),
        1
    );
    for (input, width) in [
        (format!("> ```json\n> {RAW}\n> ```\n"), 1),
        (format!("```json\n{RAW}\n```\n"), 4097),
    ] {
        let dir =
            setup(json!({"default":false,"MD013":{"code_block_line_length":width,"strict":true}}));
        fs::write(dir.path().join("doc.md"), &input).unwrap();
        let result = run(dir.path(), &["--format-code", "--write", "doc.md"], "json");
        assert_ne!(result.status.code(), Some(2));
        assert_eq!(
            fs::read_to_string(dir.path().join("doc.md")).unwrap(),
            input
        );
        assert!(calls(dir.path()).is_empty());
        assert_eq!(
            String::from_utf8_lossy(&result.stderr)
                .matches("formatter-skipped:")
                .count(),
            1
        );
    }
}

#[test]
fn formatter_blank_lines_keep_quote_and_list_container_ownership() {
    for (opening, prefix, closing) in [
        ("> ```json\n", "> ", "> ```\n"),
        ("- ```json\n", "  ", "  ```\n"),
        ("> - ```json\n", ">   ", ">   ```\n"),
    ] {
        let dir = setup(settings());
        let input = format!("{opening}{prefix}{RAW}\n{closing}");
        let expected = format!(
            "{opening}{}{closing}",
            "{\n\n  \"one\": 1,\n  \"two\": 2,\n  \"three\": 3\n}\n"
                .lines()
                .map(|line| format!("{prefix}{line}\n"))
                .collect::<String>()
        );
        fs::write(dir.path().join("doc.md"), input).unwrap();
        for _ in 0..2 {
            let result = run(
                dir.path(),
                &["--format-code", "--write", "doc.md"],
                "jsonblank",
            );
            assert_eq!(
                result.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                expected
            );
        }
    }
}
