use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(value: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(value).unwrap(),
        ..Default::default()
    }
}

#[test]
fn heading_structure_matches_pinned_findings_without_changing_source() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/heading-structure-observations.json"
    ))
    .unwrap();
    let mut mismatches = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let rule = row["rule"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"default":false,rule:row["options"]})),
        )
        .unwrap();
        assert_eq!(result.output, input, "{}", row["id"]);
        assert!(result.edits.is_empty());
        let source = Source::new(result.output.clone());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, rule);
            let span = finding.span.clone().unwrap();
            assert!(source.text().get(span.clone()).is_some());
            let line = source.position(span.start).unwrap().line;
            json!({"line":line,"severity":if finding.severity==Severity::Warning {"warning"}else{"error"}})
        }).collect();
        let expected: Vec<_> = row["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| json!({"line":f["line"],"severity":f["severity"]}))
            .collect();
        if actual != expected {
            mismatches.push(format!(
                "{}: {input:?}: {actual:?} != {expected:?}",
                row["id"]
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn disabled_heading_history_still_controls_later_increments() {
    let input = "# First\n<!-- markdownlint-disable-next-line MD001 -->\n### Third\n##### Fifth\n";
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"default":false,"MD001":true})),
    )
    .unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .message
            .contains("Expected: h4; Actual: h5")
    );
}

#[test]
fn cli_checks_and_writes_preserve_author_owned_title_decisions() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "# First\n### Third\n# Second\n";
    fs::write(directory.path().join("doc.md"), input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD001":true,"MD025":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Error MD001"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Error MD025"));
        assert_eq!(
            fs::read_to_string(directory.path().join("doc.md")).unwrap(),
            input
        );
    }
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"default":false,"MD001":"warning"}}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning MD001"));
}

#[test]
fn required_heading_patterns_do_not_silently_accept_unimplemented_regexes() {
    assert!(RuleConfig::from_value(json!({"MD043":{"headings":["% Hello.*"]}})).is_err());
}
