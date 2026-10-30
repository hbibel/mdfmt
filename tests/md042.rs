use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD042":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn empty_link_diagnostics_match_pinned_raw_source_semantics() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md042-observations.json"
    ))
    .unwrap();
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("error").is_none())
    {
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
        let actual: Vec<_> = result.diagnostics.iter().enumerate().map(|(index, finding)| {
            assert_eq!(finding.code, "MD042");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            if !input[span.clone()].contains(['\r', '\n']) {
                assert_eq!(mdfmt::utf16_len(&input[span.clone()]), row["findings"][index]["length"].as_u64().unwrap() as usize);
            }
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "context":finding.message.strip_prefix("No empty links [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
                "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
        }).collect();
        let expected: Vec<_> = row["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|finding| {
                let mut finding = finding.clone();
                finding.as_object_mut().unwrap().remove("length");
                finding
            })
            .collect();
        assert_eq!(actual, expected, "{}", row["id"]);
    }
}

#[test]
fn multiline_empty_links_report_valid_spans_instead_of_upstream_range_errors() {
    for input in ["[a\nb]()\n", "[a\r\nb]()\r\n", "> [a\n> b]()\n"] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert_eq!(first.diagnostics.len(), 1);
        let span = first.diagnostics[0].span.clone().unwrap();
        assert!(input.get(span.clone()).is_some());
        assert!(input[span].contains("b]()"));
        assert!(!first.diagnostics[0].message.contains('\n'));
        assert!(
            format_with_config(
                &first.output,
                &FormatOptions::default(),
                &config(json!(true))
            )
            .unwrap()
            .edits
            .is_empty()
        );
    }
}

#[test]
fn rule_alias_and_disabled_fixes_keep_warning_findings() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-empty-links":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "[label]()\n";
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn empty_link_positions_follow_other_safe_repairs() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD042":true})).unwrap(),
        ..Default::default()
    };
    let input = "alpha\n\n\n[label]()\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "alpha\n\n[label]()\n");
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
fn cli_reports_empty_links_without_fabricating_destinations() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    let input = "[label]()\n";
    fs::write(&path, input).unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"MD042":true}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Error MD042"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(
        &configuration,
        r#"{"fix":false,"config":{"default":false,"MD042":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD042"));
}

#[test]
fn quoted_empty_resources_ignore_continuation_container_markers() {
    for input in ["> [a](\n> )\n", "> [a](\n> <>\n> )\n"] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert_eq!(first.diagnostics.len(), 1, "{input:?}");
        let span = first.diagnostics[0].span.clone().unwrap();
        assert_eq!(&input[span], input[2..].trim_end_matches('\n'));
    }
}

#[test]
fn empty_links_inside_image_descriptions_are_not_lost_with_image_alt_text() {
    for (input, link) in [
        ("![[a]()](image)\n", "[a]()"),
        ("![[a](#)](image)\n", "[a](#)"),
        ("![prefix [a]() suffix](image)\n", "[a]()"),
    ] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert_eq!(first.diagnostics.len(), 1);
        let span = first.diagnostics[0].span.clone().unwrap();
        assert_eq!(&input[span], link);
    }
}
