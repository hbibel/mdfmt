use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD024":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn duplicate_heading_history_matches_pinned_raw_text_and_scope() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md024-observations.json"
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
            assert_eq!(finding.code, "MD024");
            let span = finding.span.clone().unwrap();
            assert!(span.is_empty());
            let line = source.position(span.start).unwrap().line;
            let context = if finding.message == "Multiple headings with the same content" {Value::Null}
                else {json!(finding.message.strip_prefix("Multiple headings with the same content [Context: \"").unwrap().strip_suffix("\"]").unwrap())};
            json!({"line":line,"context":context,"severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
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
fn rule_alias_and_disabled_fixes_keep_warning_findings_without_renaming_headings() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-duplicate-heading":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "# Same\n# Same\n";
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn approved_punctuation_repairs_can_expose_final_heading_duplicates() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD026":true,"MD024":true})).unwrap(),
        ..Default::default()
    };
    let input = "# Same!\n# Same\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "# Same\n# Same\n");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD024");
    assert_eq!(
        Source::new(result.output.clone())
            .position(result.diagnostics[0].span.clone().unwrap().start)
            .unwrap()
            .line,
        2
    );
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_reports_duplicates_without_changing_heading_text() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "# Same\n# Same\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"MD024":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Error MD024"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        &configuration,
        r#"{"fix":false,"config":{"default":false,"MD024":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD024"));
}
