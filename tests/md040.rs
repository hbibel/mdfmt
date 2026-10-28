use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD040":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn language_diagnostics_match_pinned_raw_info_and_meta_tokens() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md040-observations.json"
    ))
    .unwrap();
    let description = "Fenced code blocks should have a language specified";
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        for fix in [false, true] {
            let result = format_with_config(
                input,
                &FormatOptions { fix },
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
                    assert_eq!(finding.code, "MD040");
                    let span = finding.span.clone().unwrap();
                    assert!(span.is_empty());
                    let line = source.position(span.start).unwrap().line;
                    let suffix = finding.message.strip_prefix(description).unwrap();
                    let (detail, context) =
                        if let Some(context) = suffix.strip_prefix(" [Context: \"") {
                            (Value::Null, json!(context.strip_suffix("\"]").unwrap()))
                        } else {
                            (
                                json!(
                                    suffix
                                        .strip_prefix(" [")
                                        .unwrap()
                                        .strip_suffix(']')
                                        .unwrap()
                                ),
                                Value::Null,
                            )
                        };
                    json!({"line":line,"detail":detail,"context":context,
                    "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
                })
                .collect();
            assert_eq!(
                actual,
                *row["findings"].as_array().unwrap(),
                "{}",
                row["id"]
            );
        }
    }
}

#[test]
fn disallowed_language_and_extra_metadata_remain_two_findings() {
    let input = "```JS extra\ncode\n```\n";
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"allowed_languages":["js"],"language_only":true})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.message.contains("\"JS\" is not allowed"))
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.message.contains("more than language"))
    );
}

#[test]
fn code_language_rule_alias_and_disabled_fixes_keep_warning_findings() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"fenced-code-language":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "```\ncode\n```\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn diagnostics_refer_to_final_positions_after_safe_blank_line_repairs() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD040":true})).unwrap(),
        ..Default::default()
    };
    let input = "alpha\n\n\n```\ncode\n```\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "alpha\n\n```\ncode\n```\n");
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
fn cli_check_and_write_report_without_guessing_a_language() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let config = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "```\ncode\n```\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &config,
        r#"{"config":{"default":false,"fenced-code-language":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Error MD040"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        &config,
        r#"{"fix":false,"config":{"default":false,"MD040":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD040"));
}
