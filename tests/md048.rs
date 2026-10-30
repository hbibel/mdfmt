use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD048":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let config = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn fence_conversion_changes_only_markers_and_avoids_literal_collisions() {
    for (input, expected, options) in [
        (
            "~~~js\ncode\n~~~\n",
            "```js\ncode\n```\n",
            json!({"style":"backtick"}),
        ),
        (
            "```js\ncode\n```\n",
            "~~~js\ncode\n~~~\n",
            json!({"style":"tilde"}),
        ),
        (
            "~~~~js\ncode\n~~~~~\n",
            "````js\ncode\n`````\n",
            json!({"style":"backtick"}),
        ),
        (
            "~~~\n```\n~~~\n",
            "````\n```\n````\n",
            json!({"style":"backtick"}),
        ),
        (
            "```\n~~~\n```\n",
            "~~~~\n~~~\n~~~~\n",
            json!({"style":"tilde"}),
        ),
        (
            "> ~~~js\n> code\n> ~~~\n",
            "> ```js\n> code\n> ```\n",
            json!({"style":"backtick"}),
        ),
        (
            "- ~~~js\n  code\n  ~~~\n",
            "- ```js\n  code\n  ```\n",
            json!({"style":"backtick"}),
        ),
        ("~~~js\ncode", "```js\ncode", json!({"style":"backtick"})),
        (
            "\u{feff}~~~js\r\ncode\r\n~~~\r\n",
            "\u{feff}```js\r\ncode\r\n```\r\n",
            json!({"style":"backtick"}),
        ),
    ] {
        fixed(input, expected, options);
    }
}

#[test]
fn consistent_style_uses_disabled_reference_fences() {
    let input = "<!-- markdownlint-disable-next-line MD048 -->\n~~~\na\n~~~\n\n```\nb\n```\n";
    let expected = "<!-- markdownlint-disable-next-line MD048 -->\n~~~\na\n~~~\n\n~~~\nb\n~~~\n";
    fixed(input, expected, json!(true));
}

#[test]
fn unsafe_info_and_uncertain_or_disabled_boundaries_remain_unchanged() {
    for input in [
        "~~~js `meta`\ncode\n~~~\n",
        ">\t~~~js\n>\tcode\n>\t~~~\n",
        "~~~\ncode\n<!-- markdownlint-disable-next-line MD048 -->\n~~~\n",
    ] {
        let config = config(json!({"style":"backtick"}));
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(result.output, input);
        assert!(!result.diagnostics.is_empty());
    }
    let input = "~~~\ncode\n<!-- markdownlint-disable-next-line MD048 -->\n~~~\n";
    assert!(
        !config(json!({"style":"backtick"}))
            .document(input)
            .unwrap()
            .state("MD048", 4)
            .unwrap()
            .enabled
    );
}

#[test]
fn diagnostics_match_pinned_opening_fence_styles() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md048-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD048");
            let span = finding.span.clone().unwrap();
            assert!(span.is_empty());
            json!({"line":source.position(span.start).unwrap().line,
                "detail":finding.message.strip_prefix("Code fence style [").unwrap().strip_suffix(']').unwrap(),
                "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
        }).collect();
        assert_eq!(
            actual,
            *row["findings"].as_array().unwrap(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn disabled_fixes_report_alias_warnings_without_touching_code() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"code-fence-style":{"style":"backtick","severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "~~~js\ncode\n~~~\n";
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn fence_repairs_compose_with_eof_and_leave_literal_findings() {
    let config = ResolvedConfig {rules:RuleConfig::from_value(json!({"default":false,"MD048":{"style":"backtick"},"MD010":true,"MD040":{"allowed_languages":["rust"]},"MD047":true})).unwrap(), ..Default::default()};
    let input = "~~~js\ncode\tvalue\n~~~";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "```js\ncode\tvalue\n```\n");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|finding| finding.code.as_str())
            .collect::<Vec<_>>(),
        vec!["MD040", "MD010"]
    );
    let mut applied = String::new();
    let mut cursor = 0;
    for edit in &result.edits {
        applied.push_str(&input[cursor..edit.span.start]);
        applied.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    applied.push_str(&input[cursor..]);
    assert_eq!(applied, result.output);
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_normalizes_fences_without_formatting_code_contents() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "~~~js\ncode\tvalue\n~~~\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"code-fence-style":{"style":"backtick"}}}"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "```js\ncode\tvalue\n```\n"
    );
    fs::write(&path, input).unwrap();
    fs::write(&configuration, r#"{"fix":false,"config":{"default":false,"MD048":{"style":"backtick","severity":"warning"}}}"#).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD048"));
}

#[test]
fn target_marker_info_prefix_does_not_absorb_metadata_or_block_other_repairs() {
    let input = "```~~~js\ncode\n```\n\n```rust\nok\n```\n";
    let expected = "```~~~js\ncode\n```\n\n~~~rust\nok\n~~~\n";
    fixed(input, expected, json!({"style":"tilde"}));
}
