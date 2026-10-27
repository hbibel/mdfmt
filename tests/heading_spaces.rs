use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(rules: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    }
}

#[test]
fn source_separators_are_normalized_without_changing_the_first_unit() {
    let config = config(json!({"default":false,"MD019":true,"MD021":true}));
    for (input, expected) in [
        ("#  Hello", "# Hello"),
        ("#  Hello  ###", "# Hello ###"),
        ("#\t Hello", "#\tHello"),
        ("# \tHello", "# Hello"),
        ("#\tHello", "#\tHello"),
        ("#   ", "# "),
        ("#  ##", "# ##"),
        ("  #  Hello", "  # Hello"),
        ("> #  Hello  ###", "> # Hello ###"),
        ("- #  Hello", "- # Hello"),
        ("\u{feff}#  😀Hello  ###\r\n", "\u{feff}# 😀Hello ###\r\n"),
        ("#foo!\n---\n", "#foo!\n---\n"),
        ("```\n#  Hello\n```", "```\n#  Hello\n```"),
        (
            "#  <!-- markdownlint-disable-line MD026 -->",
            "# <!-- markdownlint-disable-line MD026 -->",
        ),
    ] {
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected, "{input:?}");
        assert!(first.diagnostics.is_empty());
        let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, first.output);
        assert!(second.edits.is_empty());
    }
}

#[test]
fn open_and_closed_styles_obey_independent_rule_permissions() {
    let config = config(json!({"default":false,"MD019":true,"MD021":false}));
    let input = "#  Open\n\n#  Closed  ###\n";
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        "# Open\n\n#  Closed  ###\n"
    );
    let config =
        self::config(json!({"default":false,"MD019":false,"no-multiple-space-closed-atx":true}));
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        "#  Open\n\n# Closed ###\n"
    );
    let input = "<!-- markdownlint-disable-file spaces -->\n#  Open\n\n#  Closed  ###\n";
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        input
    );
}

#[test]
fn disabled_fixes_report_output_safe_spans_and_configured_severity() {
    let config = config(json!({"default":false,"MD021":{"severity":"warning"}}));
    let input = "#  😀Hello  ###";
    let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 2);
    for finding in &result.diagnostics {
        assert_eq!(finding.code, "MD021");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(&input[finding.span.clone().unwrap()], " ");
    }
    assert_eq!(
        Source::new(input.into())
            .position(result.diagnostics[1].span.as_ref().unwrap().start)
            .unwrap()
            .column_utf16,
        12
    );
}

#[test]
fn spacing_findings_and_fixes_match_recorded_upstream_evidence() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/heading-spaces-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let config = config(row["config"].clone());
        let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
        let source = Source::new(input.into());
        let findings: Vec<_> = result.diagnostics.iter().map(|finding| {
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"rule":finding.code,"line":position.line,
                "column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
        }).collect();
        assert_eq!(
            findings,
            *row["findings"].as_array().unwrap(),
            "{}",
            row["id"]
        );
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(
            first.output,
            row["output"].as_str().unwrap(),
            "{}",
            row["id"]
        );
        assert!(
            format_with_config(&first.output, &FormatOptions::default(), &config)
                .unwrap()
                .edits
                .is_empty()
        );
    }
}

#[test]
fn heading_style_exposure_and_all_rule_stages_settle_with_original_coordinates() {
    let config = config(
        json!({"default":false,"MD012":true,"MD013":{"line_length":10},"MD019":true,"MD021":true,"MD026":true,"MD047":true}),
    );
    let input = "#  Hi!  ###!\n\n\nalpha beta gamma";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "# Hi ###\n\nalpha beta\ngamma\n");
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
