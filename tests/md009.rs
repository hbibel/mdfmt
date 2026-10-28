use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD009":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let config = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn incidental_suffix_whitespace_is_removed_with_configured_break_counts() {
    for (input, expected, options) in [
        ("alpha ", "alpha", json!(true)),
        ("alpha \r\n", "alpha\r\n", json!(true)),
        ("alpha\t\n", "alpha\n", json!(true)),
        ("alpha   ", "alpha", json!(true)),
        ("alpha  ", "alpha  ", json!(true)),
        ("alpha  \n", "alpha\n", json!({"br_spaces":0})),
        ("alpha  \n", "alpha\n", json!({"br_spaces":1})),
        ("alpha   ", "alpha   ", json!({"br_spaces":3})),
        ("\u{feff}alpha \r\n", "\u{feff}alpha\r\n", json!(true)),
    ] {
        fixed(input, expected, options);
    }
}

#[test]
fn hard_breaks_and_protected_or_meaningful_source_are_preserved() {
    for (input, options) in [
        ("alpha   \nbeta\n", json!(true)),
        ("alpha  \nbeta\n", json!({"br_spaces":0})),
        ("`alpha  \nbeta`\n", json!({"strict":true})),
        ("# Heading \n", json!(true)),
        ("```\ncode \n```\n", json!({"code_blocks":true})),
        ("| a | \n| - | \n| b | \n", json!(true)),
        ("alpha\u{a0}\n", json!(true)),
        ("alpha\u{2028}\n", json!(true)),
        ("alpha\\ \nbeta\n", json!(true)),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(options)).unwrap();
        assert_eq!(result.output, input, "{input:?}");
        assert!(!result.diagnostics.is_empty(), "{input:?}");
    }
}

#[test]
fn diagnostic_options_and_source_positions_match_pinned_runtime_evidence() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md009-observations.json"
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
        let source = Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD009");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Trailing spaces [").unwrap().strip_suffix(']').unwrap(),
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
fn line_scoped_overrides_are_not_changed_by_reflow() {
    let input = "<!-- markdownlint-disable-next-line MD009 -->\nalpha beta gamma ";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD009":true,"MD013":{"line_length":10}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD009":true,"MD013":true})).unwrap(),
        ..Default::default()
    };
    let input = "<!-- markdownlint-disable-next-line MD009 -->\nalpha\nbeta ";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(
        first.output,
        "<!-- markdownlint-disable-next-line MD009 -->\nalpha\nbeta"
    );
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn disabled_fixes_report_warnings_without_rewriting_input() {
    let config = config(json!({"severity":"warning"}));
    let first = format_with_config("alpha ", &FormatOptions { fix: false }, &config).unwrap();
    assert_eq!(first.output, "alpha ");
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].severity, Severity::Warning);
    assert_eq!(first.diagnostics[0].span, Some(5..6));
    let mut config = config;
    config.fix = false;
    assert_eq!(
        format_with_config("alpha ", &FormatOptions::default(), &config)
            .unwrap()
            .output,
        "alpha "
    );
}

#[test]
fn incidental_deletions_compose_with_all_later_rule_stages() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD004":{"style":"plus"},"MD009":true,"MD012":true,"MD013":{"line_length":10},"MD019":true,"MD021":true,"MD026":true,"MD035":{"style":"* * *"},"MD047":true})).unwrap(), ..Default::default() };
    let input = "#  Title!\n\n\n- alpha beta gamma \n\n---";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(
        first.output,
        "# Title\n\n+ alpha\n  beta\n  gamma\n\n* * *\n"
    );
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
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}
