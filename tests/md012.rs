use mdfmt::{
    FormatOptions, Severity,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn excess_blank_lines_are_removed_without_changing_blocks() {
    for (input, expected, options) in [
        ("alpha\n\n\n# beta\n", "alpha\n\n# beta\n", json!(true)),
        ("alpha\n\n", "alpha\n", json!(true)),
        ("\n\nalpha\n", "\nalpha\n", json!(true)),
        ("\n\n", "", json!(true)),
        ("\u{feff}\n\nalpha\n", "\u{feff}\nalpha\n", json!(true)),
        (
            "alpha\r\n\r\n\r\n# beta\r\n",
            "alpha\r\n\r\n# beta\r\n",
            json!(true),
        ),
        ("alpha\n \n\t\n# beta\n", "alpha\n \n# beta\n", json!(true)),
        (
            "alpha\n\n\n# beta\n",
            "alpha\n\n\n# beta\n",
            json!({"maximum":2}),
        ),
    ] {
        let config = config(options);
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected, "{input:?}");
        let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, first.output);
        assert!(second.edits.is_empty());
    }
}

#[test]
fn protected_source_and_semantic_block_boundaries_are_not_repaired() {
    for input in [
        "alpha\n\nbeta\n",
        "- alpha\n\n- beta\n",
        "- alpha\n\n\n- beta\n",
        "```\n\n\n```\n",
        "    alpha\n\n\n    beta\n",
        "<div>\n\n\nhello\n</div>\n",
        "$$\n\n\nx\n$$\n",
        "::note\n\n\ntext\n",
        "---\ntitle: hi\n---\n\n\nhello",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input, "{input:?}");
    }
}

#[test]
fn blank_line_findings_match_recorded_upstream_runtime_evidence() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md012-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        if row["options"]["maximum"] == 0 {
            assert!(RuleConfig::from_value(json!({"MD012":row["options"]})).is_err());
            continue;
        }
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        let source = mdfmt::Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            json!({"line":source.position(finding.span.as_ref().unwrap().start).unwrap().line,
                "detail":finding.message.strip_prefix("Multiple consecutive blank lines [").unwrap().strip_suffix(']').unwrap(),
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
fn disabled_fixes_report_expected_actual_counts_and_severity() {
    let input = "alpha\n\n\n# beta\n";
    let result = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"severity":"warning"})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD012");
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    assert!(
        result.diagnostics[0]
            .message
            .contains("Expected: 1; Actual: 2")
    );
}

#[test]
fn final_newline_and_blank_line_rules_are_stable_together() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD047":true})).unwrap(),
        ..Default::default()
    };
    let first = format_with_config("alpha\n\n", &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "alpha\n");
    assert!(first.diagnostics.is_empty());
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn directives_and_rule_disabling_prevent_blank_line_edits() {
    let input = "<!-- markdownlint-disable-file MD012 -->\n\n\nalpha\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.is_empty());
    let input = "alpha\n\n\n# beta\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(false))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn all_implemented_rules_produce_stable_output_and_original_edit_coordinates() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD013":{"line_length":10},"MD026":true,"MD047":true})).unwrap(), ..Default::default() };
    let input = "# Title!\n\n\nalpha beta gamma";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "# Title\n\nalpha beta\ngamma\n");
    assert!(first.diagnostics.is_empty());
    let mut applied = String::new();
    let mut cursor = 0;
    for edit in &first.edits {
        applied.push_str(&input[cursor..edit.span.start]);
        applied.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    applied.push_str(&input[cursor..]);
    assert_eq!(applied, first.output);
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}
