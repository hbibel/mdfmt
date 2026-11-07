use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD033":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn opening_html_tokens_match_pinned_ranges_and_allowance_contexts() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md033-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input, "{}", row["id"]);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code,"MD033");
            let span=finding.span.clone().unwrap();
            assert!(input.get(span.clone()).is_some());
            let position=source.position(span.start).unwrap();
            let column=position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}'));
            let detail=finding.message.strip_prefix("Inline HTML [").unwrap().strip_suffix(']').unwrap();
            json!({"line":position.line,"range":[column,mdfmt::utf16_len(&input[span])],"detail":detail,"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        if actual != *row["findings"].as_array().unwrap() {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn only_opening_tags_are_findings_and_html_is_never_deleted() {
    let input = "<b>x</b>\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(0..3));
    assert_eq!(result.diagnostics[0].message, "Inline HTML [Element: b]");
    assert!(
        format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"allowed_elements":["B"]}))
        )
        .unwrap()
        .diagnostics
        .is_empty()
    );
}

#[test]
fn explicitly_empty_table_allowances_do_not_inherit_ordinary_allowances() {
    let input = "<b>ordinary</b>\n\n| h |\n| --- |\n| <b>table</b> |\n";
    assert!(
        format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"allowed_elements":["B"]}))
        )
        .unwrap()
        .diagnostics
        .is_empty()
    );
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"allowed_elements":["B"],"table_allowed_elements":[]})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    let start = input.rfind("<b>").unwrap();
    assert_eq!(result.diagnostics[0].span, Some(start..start + 3));
}

#[test]
fn html_findings_use_positions_after_other_approved_repairs() {
    let input = "# Heading!\n\n\n<b>x</b>\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD033":true,"MD026":true})).unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "# Heading\n\n\n<b>x</b>\n");
    assert_eq!(result.diagnostics.len(), 1);
    let source = Source::new(result.output.clone());
    assert_eq!(
        source
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        4
    );
    assert_eq!(
        result.diagnostics[0].span.as_ref().unwrap().start,
        result.output.find("<b>").unwrap()
    );
    let again = format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
}

#[test]
fn cli_reports_html_without_rewriting_source_in_check_or_write_modes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "<b>x</b>\n";
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD033":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Error MD033"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"default":false,"MD033":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning MD033"));
}
