use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD045":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn image_diagnostics_match_pinned_markdown_and_html_tokens() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md045-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        let actual: Vec<_> = result
            .diagnostics
            .iter()
            .map(|finding| {
                assert_eq!(finding.code, "MD045");
                assert_eq!(
                    finding.message,
                    "Images should have alternate text (alt text)"
                );
                let span = finding.span.clone().unwrap();
                let position = source.position(span.start).unwrap();
                let range = if span.is_empty() {
                    Value::Null
                } else {
                    json!([
                        position.column_utf16
                            - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                        mdfmt::utf16_len(&input[span])
                    ])
                };
                json!({"line":position.line,"range":range,
                "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
            })
            .collect();
        let mut expected = row["findings"].as_array().unwrap().clone();
        expected.sort_by_key(|finding| {
            (
                finding["line"].as_u64().unwrap(),
                finding["range"][0].as_u64().unwrap_or(1),
            )
        });
        assert_eq!(actual, expected, "{}", row["id"]);
    }
}

#[test]
fn rule_alias_and_disabled_fixes_keep_warning_findings() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-alt-text":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "![](image)\n";
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn image_positions_follow_other_safe_repairs_without_inventing_descriptions() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD045":true})).unwrap(),
        ..Default::default()
    };
    let input = "alpha\n\n\n![](image)\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "alpha\n\n![](image)\n");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(result.output.clone())
            .position(result.diagnostics[0].span.clone().unwrap().start)
            .unwrap()
            .line,
        3
    );
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_reports_missing_alt_text_without_rewriting_images() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "<img src=image>\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"MD045":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Error MD045"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        &configuration,
        r#"{"fix":false,"config":{"default":false,"MD045":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD045"));
}
