use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD011":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn reversed_delimiters_are_findings_not_permission_to_create_links() {
    let input = "(text)[url]\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD011");
    assert_eq!(first.diagnostics[0].span, Some(0..11));
}

#[test]
fn source_matching_and_literal_exclusions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md011-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|finding|{
            assert_eq!(finding.code,"MD011");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Reversed link syntax [").unwrap().strip_suffix(']').unwrap(),
                "severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
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
fn inline_literals_after_html_container_recovery_stay_excluded() {
    for literal in ["`(text)[url]`", "$(text)[url]$", "``(text)[url]``"] {
        let input = format!("- item\n<span>\n{literal}\n(real)[url]\n");
        let first =
            format_with_config(&input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert_eq!(first.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            Source::new(input.clone())
                .position(first.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            4
        );
    }
}

#[test]
fn directives_warnings_and_final_output_reflow_are_stable() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"no-reversed-links":{"severity":"warning"},"MD013":{"line_length":20}})).unwrap(),..Default::default()};
    let input = "<!-- markdownlint-disable-next-line MD011 -->\n(disabled)[url]\n\nlong prose before (real)[url] after words\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    let findings: Vec<_> = first
        .diagnostics
        .iter()
        .filter(|finding| finding.code == "MD011")
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warning);
    assert!(!first.output.contains("[real](url)"));
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn cli_preserves_reversed_link_text_in_check_and_write_modes() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD011":true}}"#,
    )
    .unwrap();
    fs::write(&path, "(text)[url]\r\n").unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), "(text)[url]\r\n");
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD011"));
    }
}
