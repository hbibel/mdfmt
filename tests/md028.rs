use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD028":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn quote_gap_findings_match_pinned_sibling_and_container_semantics() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md028-observations.json"
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
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD028");
            assert_eq!(finding.message, "Blank line inside blockquote");
            let span = finding.span.clone().unwrap();
            assert!(span.is_empty());
            let position = source.position(span.start).unwrap();
            assert_eq!(position.column_utf16, 1);
            json!({"line":position.line,"severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
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
fn alias_and_disabled_fixes_keep_warning_findings_without_merging_quotes() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-blanks-blockquote":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "> one\n\n> two\n";
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn gap_findings_refer_to_retained_blanks_after_md012_repairs() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD028":true})).unwrap(),
        ..Default::default()
    };
    let input = "> one\n\n\n> two\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "> one\n\n> two\n");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(6..6));
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_reports_quote_gaps_without_combining_quotations() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "> one\n\n> two\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"MD028":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Error MD028"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        &configuration,
        r#"{"fix":false,"config":{"default":false,"MD028":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD028"));
}
