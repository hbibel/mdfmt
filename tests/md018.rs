use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD018":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn missing_separators_are_findings_not_permission_to_create_headings() {
    let input = "#Heading\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD018");
    assert_eq!(first.diagnostics[0].span, Some(0..2));
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config(json!(true)))
            .unwrap()
            .output,
        input
    );
}

#[test]
fn source_checks_and_flow_exclusions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md018-observations.json"
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
            assert_eq!(finding.code,"MD018");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            let width=mdfmt::utf16_len(&input[span]);
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),
                "length":if input.starts_with("#😀"){width-1}else{width},
                "context":finding.message.strip_prefix("No space after hash on atx style heading [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
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
fn math_bodies_do_not_leak_code_or_html_flow_past_the_closing_fence() {
    for input in [
        "$$\n```\n$$\n#Heading\n",
        "$$\n<div>\n$$\n#Heading\n",
        "$$ info\n#Hidden\n$$\n#Heading\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            Source::new(input.into())
                .position(result.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            4
        );
    }
}

#[test]
fn unicode_ranges_and_disabled_or_warning_states_stay_source_backed() {
    let first = format_with_config(
        "\u{feff}#😀\r\n",
        &FormatOptions::default(),
        &config(json!({"severity":"warning"})),
    )
    .unwrap();
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].severity, Severity::Warning);
    assert_eq!(first.diagnostics[0].span, Some(3..8));
    let input = "<!-- markdownlint-disable-next-line MD018 -->\n#Heading\n#Second\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.output, input);
}

#[test]
fn cli_write_preserves_heading_meaning_and_reports_remaining_errors() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"no-missing-space-atx":true}}"#,
    )
    .unwrap();
    fs::write(&path, "#Heading\r\n").unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), "#Heading\r\n");
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD018"));
    }
}

#[test]
fn final_output_findings_survive_reflow_and_line_permissions() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD018":true,"MD013":{"line_length":12}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input =
        "<!-- markdownlint-disable-next-line MD018 -->\n#alpha beta gamma\n\n#delta epsilon zeta\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(
        first
            .diagnostics
            .iter()
            .filter(|finding| finding.code == "MD018")
            .count(),
        1
    );
    assert!(first.output.contains("#delta"));
    assert!(!first.output.contains("# delta"));
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn list_owned_math_cannot_consume_unindented_root_candidates() {
    for input in [
        "- item\n  $$\n#Heading\n",
        "- item\n  $$\n#Heading\n$$\n#After\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(
            result.diagnostics.len(),
            1,
            "{input:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            Source::new(input.into())
                .position(result.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            3
        );
    }
}

#[test]
fn math_heavy_documents_do_not_reparse_once_per_math_block() {
    use std::time::{Duration, Instant};
    for options in [json!(false), json!(true)] {
        let input = format!("{}#Heading\n", "$$\nx\n$$\n\n".repeat(500));
        let start = Instant::now();
        let result = format_with_config(
            &input,
            &FormatOptions { fix: false },
            &config(options.clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert_eq!(
            result.diagnostics.len(),
            usize::from(options == json!(true))
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "500 math blocks took {:?}",
            start.elapsed()
        );
    }
}

#[test]
fn generic_html_after_a_container_does_not_hide_later_root_lines() {
    for input in ["- item\n<span>\n#Heading\n", "> text\n<span>\n#Heading\n"] {
        let first =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(first.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            Source::new(input.into())
                .position(first.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            3
        );
    }
}
