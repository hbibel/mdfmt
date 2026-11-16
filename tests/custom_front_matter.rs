use mdfmt::{
    FormatOptions, Source,
    config::{FrontMatterPattern, ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
use std::{fs, process::Command};
#[cfg(unix)]
use std::{path::PathBuf, sync::OnceLock};

const PATTERN: &str = r"^%%%\n[\s\S]*?\n%%%\n";
const ASSERTED_PATTERN: &str = r"^%%%\n[\s\S]*?\n%%%\n\B";
fn config(pattern: &str, rules: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        front_matter: Some(FrontMatterPattern::new(pattern).unwrap()),
        ..Default::default()
    }
}

#[test]
fn metadata_prefix_boundaries_preserve_original_coordinates() {
    for newline in ["\n", "\r\n", "\r"] {
        for bom in ["", "\u{feff}"] {
            let prefix = format!("{bom}%%%\nname: 😀\n# Secret!\n%%%\n").replace('\n', newline);
            let input = format!("{prefix}{newline}# Visible!{newline}");
            let config = config(PATTERN, json!({"default":false,"MD026":true}));
            assert_eq!(
                config
                    .front_matter
                    .as_ref()
                    .unwrap()
                    .prefix_end(&input)
                    .unwrap(),
                prefix.len()
            );
            let result = format_with_config(&input, &FormatOptions::default(), &config).unwrap();
            assert_eq!(
                result.output,
                format!("{prefix}{newline}# Visible{newline}")
            );
            assert_eq!(result.edits.len(), 1);
            assert_eq!(&input[result.edits[0].span.clone()], "!");
            let mut rebuilt = input.clone();
            for edit in result.edits.iter().rev() {
                rebuilt.replace_range(edit.span.clone(), &edit.replacement);
            }
            assert_eq!(rebuilt, result.output);
            let again =
                format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
            assert_eq!(again.output, result.output);
            assert!(again.edits.is_empty());
        }
    }
    let default_like = "---\n# Secret!\n---\n\n# Visible!\n";
    for pattern in ["^NOT-PRESENT", "^"] {
        let settings = config(pattern, json!({"default":false,"MD026":true}));
        let output =
            format_with_config(default_like, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(output.output, "---\n# Secret\n---\n\n# Visible\n");
    }
    let input = "META:# Visible!\r\n";
    let settings = config("^META:", json!({"default":false,"MD026":true}));
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, "META:# Visible\r\n");
    let diagnostic = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    let position = Source::new(input.into())
        .position(diagnostic.diagnostics[0].span.as_ref().unwrap().start)
        .unwrap();
    assert_eq!(position.line, 1);
}

#[test]
fn metadata_context_survives_all_reparse_stages() {
    let prefix = "%%%\n# Secret!\nkey:\tvalue\n| a | b |\n| - | - |\n| bad |\n```json\n{\"one\": 1, \"two\": 2, \"three\": 3}\n```\n[hidden](#MISSING)\n%%%\n";
    let body = "\n # Body!\n\n-   one two three four five six seven eight nine ten\n\n[jump](#BODY)\n\n```json\n{\"one\": 1, \"two\": 2, \"three\": 3}\n```\n";
    let all_input = format!("{prefix}\n# Visible\n");
    let all = format_with_config(
        &all_input,
        &FormatOptions::default(),
        &config(PATTERN, json!({})),
    )
    .unwrap();
    assert_eq!(all.output, all_input);
    assert!(all.diagnostics.is_empty(), "{:?}", all.diagnostics);
    let settings = config(
        PATTERN,
        json!({"default":false,"MD013":{"line_length":28,"strict":true},"MD026":true,"MD023":true,"MD004":{"style":"plus"},"MD030":true,"MD048":{"style":"tilde"},"MD051":true,"MD056":true}),
    );
    let result = format_with_config(
        &format!("{prefix}{body}"),
        &FormatOptions::default(),
        &settings,
    )
    .unwrap();
    assert!(result.output.starts_with(prefix));
    let body = &result.output[prefix.len()..];
    assert!(body.contains("# Body\n"), "{body}");
    assert!(body.contains("+ one two three four five"), "{body}");
    assert!(body.contains("[jump](#body)"), "{body}");
    assert!(body.contains("~~~json\n"), "{body}");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|finding| finding.code != "MD056" && finding.code != "MD051")
    );
    assert!(
        result
            .edits
            .iter()
            .all(|edit| edit.span.start >= prefix.len())
    );
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert!(again.edits.is_empty());
    let input = "%%%\nname: Metadata title\n%%%\n\nOrdinary prose.\n";
    let settings = config(
        PATTERN,
        json!({"default":false,"MD041":{"front_matter_title":"^name:"}}),
    );
    assert!(
        format_with_config(input, &FormatOptions { fix: false }, &settings)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    let input = "%%%\n<!-- markdownlint-disable MD026 -->\n%%%\n\n# Body!\n";
    assert_eq!(
        format_with_config(
            input,
            &FormatOptions::default(),
            &config(PATTERN, json!({"default":false,"MD026":true}))
        )
        .unwrap()
        .output,
        input
    );
}

#[test]
fn body_repairs_cannot_change_metadata_recognition() {
    for (pattern, prefix, body) in [
        (ASSERTED_PATTERN, "%%%\n# Secret!\n%%%\n", "*body*\n"),
        (
            r"^%%%\n[\s\S]*?\n%%%\n\b",
            "%%%\n# Secret!\n%%%\n",
            "_body_\n",
        ),
        (
            r"^%%%\n[\s\S]*?\n%%%\n\b",
            "",
            "%%%\n# Visible!\n%%%\n*body*\n",
        ),
    ] {
        let input = format!("{prefix}{body}");
        let style = if body.starts_with('_') {
            "asterisk"
        } else {
            "underscore"
        };
        let rules = json!({"default":false,"MD026":true,"MD049":{"style":style}});
        let settings = config(pattern, rules.clone());
        let expected = if prefix.is_empty() {
            input.replace("Visible!", "Visible")
        } else {
            input.clone()
        };
        let checked = format_with_config(&input, &FormatOptions { fix: false }, &settings).unwrap();
        assert!(
            checked
                .diagnostics
                .iter()
                .all(|d| d.span.as_ref().unwrap().start >= prefix.len())
        );
        let result = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, expected, "{pattern}");
        assert!(result.diagnostics.iter().any(|d| d.code == "MD049"));
        let again =
            format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, result.output);
        assert!(again.edits.is_empty());
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("doc.md"), &input).unwrap();
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"frontMatter":pattern,"config":rules}).to_string(),
        )
        .unwrap();
        for mode in ["--check", "--write", "--write"] {
            let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args([mode, "doc.md"])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(1),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                if mode == "--check" { &input } else { &expected }.to_string()
            );
        }
    }
}

#[test]
fn pinned_unicode_properties_preserve_metadata_on_write() {
    for ch in ['\u{1e6c0}', '\u{10940}', '\u{11db0}'] {
        for property in [r"\p{L}", r"\P{C}", r"\P{Cn}", r"[\p{L}]"] {
            let pattern = format!("^{property}[\\s\\S]*?END\\n");
            let prefix = format!("{ch}\n# Secret!\nEND\n");
            let input = format!("{prefix}\n# Body!\n");
            let settings = config(&pattern, json!({"default":false,"MD026":true}));
            assert_eq!(
                settings
                    .front_matter
                    .as_ref()
                    .unwrap()
                    .prefix_end(&input)
                    .unwrap(),
                prefix.len(),
                "{property} U+{:X}",
                u32::from(ch)
            );
            let checked =
                format_with_config(&input, &FormatOptions { fix: false }, &settings).unwrap();
            assert_eq!(checked.diagnostics.len(), 1);
            assert_eq!(
                checked.diagnostics[0].span,
                Some(prefix.len() + 7..prefix.len() + 8)
            );
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("doc.md"), &input).unwrap();
            fs::write(
                dir.path().join(".markdownlint-cli2.jsonc"),
                json!({"frontMatter":pattern,"config":{"default":false,"MD026":true}}).to_string(),
            )
            .unwrap();
            for _ in 0..2 {
                let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(dir.path())
                    .args(["--write", "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                    format!("{prefix}\n# Body\n")
                );
            }
        }
    }
}

#[test]
fn re2_invalid_patterns_fail_before_output_or_write() {
    let dir = tempfile::tempdir().unwrap();
    let input = "%%%Title\n# Body!\n";
    fs::write(dir.path().join("doc.md"), input).unwrap();
    for pattern in [
        r"^%%%\p{Letter}+\n",
        r"\p{Lowercase_Letter}",
        r"\p{ASCII}",
        r"\p{Script=Greek}",
        "a{1001}",
        "a{0,1001}",
        "(?:a{100}){11}",
    ] {
        assert!(FrontMatterPattern::new(pattern).is_err(), "{pattern}");
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"frontMatter":pattern,"config":{"default":false,"MD026":true}}).to_string(),
        )
        .unwrap();
        for mode in ["--check", "--write"] {
            let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args([mode, "doc.md"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{pattern}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("frontMatter"));
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                input
            );
        }
    }
    assert!(FrontMatterPattern::new(r"^%%%\p{L}+\n").is_ok());
}

#[test]
fn configured_metadata_matches_pinned_re2_boundaries() {
    let rows: Value =
        serde_json::from_str(include_str!("fixtures/front-matter/cases.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let Some(pattern) = row
            .get("pattern")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if row["source"].get("error").is_some() {
            assert!(FrontMatterPattern::new(pattern).is_err(), "{}", row["id"]);
            continue;
        }
        let compiled = FrontMatterPattern::new(pattern).unwrap();
        let input = row["input"].as_str().unwrap();
        let end = compiled.prefix_end(input).unwrap();
        let actual = input[..end]
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        assert_eq!(
            actual,
            row["source"]["normalizedPrefix"].as_str().unwrap_or(""),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn cli_pattern_precedence_defaults_and_errors_preserve_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let input = "%%%\n# Secret!\n%%%\n\n# Body!\n";
    fs::write(dir.path().join("doc.md"), input).unwrap();
    fs::write(
        dir.path().join("base.json"),
        json!({"frontMatter":"^ABSENT","config":{"default":false,"MD026":true}}).to_string(),
    )
    .unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"frontMatter":PATTERN}).to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--config", "base.json", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "%%%\n# Secret!\n%%%\n\n# Body\n"
    );
    for invalid in [json!(true), json!("["), json!("(?=x)x")] {
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"frontMatter":invalid,"config":{"default":false,"MD026":true}}).to_string(),
        )
        .unwrap();
        for mode in ["--check", "--write"] {
            let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args([mode, "doc.md"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("frontMatter"));
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                input
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn metadata_fences_never_launch_formatters_but_body_fences_still_do() {
    static BUILD: OnceLock<tempfile::TempDir> = OnceLock::new();
    let build = BUILD.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let status = Command::new("rustc")
            .args(["--edition=2024", "-O"])
            .arg(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/support/code_formatter_fixture.rs"),
            )
            .arg("-o")
            .arg(directory.path().join("prettier"))
            .status()
            .unwrap();
        assert!(status.success());
        directory
    });
    let dir = tempfile::tempdir().unwrap();
    let prefix = "%%%\n```json\n{\"one\": 1, \"two\": 2, \"three\": 3}\n```\n%%%\n";
    fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"frontMatter":PATTERN,"config":{"default":false,"MD013":{"strict":true,"code_block_line_length":20}}}).to_string()).unwrap();
    let input = format!("{prefix}\nPlain body.\n");
    fs::write(dir.path().join("doc.md"), &input).unwrap();
    let run = |mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args(["--format-code", "--write", "doc.md"])
            .env("PATH", build.path())
            .env("MDFMT_FIXTURE_MODE", mode)
            .env("MDFMT_FIXTURE_LOG", dir.path().join("calls"))
            .output()
            .unwrap()
    };
    let first = run("json");
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        input
    );
    assert!(!dir.path().join("calls").exists());
    fs::write(
        dir.path().join("doc.md"),
        format!("{prefix}\n```json\n{{\"one\": 1, \"two\": 2, \"three\": 3}}\n```\n"),
    )
    .unwrap();
    let second = run("json");
    assert_eq!(
        second.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let output = fs::read_to_string(dir.path().join("doc.md")).unwrap();
    assert!(output.starts_with(prefix));
    assert!(output[prefix.len()..].contains("{\n  \"one\": 1,"));
    assert_eq!(
        fs::read_to_string(dir.path().join("calls"))
            .unwrap()
            .matches("CALL ")
            .count(),
        2
    );
    let failed = run("partial");
    assert_eq!(failed.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("doc.md:7:1: Warning formatter-failed:")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        output
    );
    fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"frontMatter":ASSERTED_PATTERN,"config":{"default":false,"MD049":{"style":"underscore"},"MD013":{"strict":true,"code_block_line_length":20}}}).to_string()).unwrap();
    let input = format!("{prefix}*body*\n");
    fs::write(dir.path().join("doc.md"), &input).unwrap();
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    let asserted = run("json");
    assert_eq!(
        asserted.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&asserted.stderr)
    );
    assert_eq!(fs::read_to_string(dir.path().join("calls")).unwrap(), calls);
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        input
    );
}
