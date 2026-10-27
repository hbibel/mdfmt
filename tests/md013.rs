use mdfmt::{
    Diagnostic, FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn findings(text: &str, options: Value) -> Vec<Diagnostic> {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD013":options})).unwrap(),
        ..Default::default()
    };
    format_with_config(text, &FormatOptions { fix: false }, &config)
        .unwrap()
        .diagnostics
}

fn lines(text: &str, options: Value) -> Vec<usize> {
    let source = Source::new(text.into());
    findings(text, options)
        .iter()
        .map(|d| {
            assert_eq!(d.code, "MD013");
            assert_eq!(d.rule_id.as_deref(), Some("MD013"));
            source
                .position(d.span.as_ref().unwrap().start)
                .unwrap()
                .line
        })
        .collect()
}

#[test]
fn default_strict_and_stern_use_their_distinct_lint_exceptions() {
    let text = "one two three\nverylongtoken\n> verylongtoken\n- verylongtoken\n";
    assert_eq!(lines(text, json!({"line_length":9})), Vec::<usize>::new());
    assert_eq!(
        lines(text, json!({"line_length":9,"stern":true})),
        vec![1, 4]
    );
    assert_eq!(
        lines(text, json!({"line_length":9,"strict":true})),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        lines("alpha beta\n", json!({"line_length":10,"strict":true})),
        Vec::<usize>::new()
    );
}

#[test]
fn category_widths_inherit_and_include_flags_override_strict() {
    let text = "# heading long\n\nsetext heading\n--------------\n\n```text\nlong code line\n```\n\n| long cell |\n| --------- |\n| data data |\n";
    assert_eq!(
        lines(text, json!({"line_length":10,"strict":true})),
        vec![1, 3, 4, 7, 10, 11, 12]
    );
    assert_eq!(
        lines(
            text,
            json!({"line_length":10,"heading_line_length":30,"code_block_line_length":30,"strict":true})
        ),
        vec![10, 11, 12]
    );
    assert_eq!(
        lines(
            text,
            json!({"line_length":10,"headings":false,"code_blocks":false,"tables":false,"strict":true})
        ),
        Vec::<usize>::new()
    );
}

#[test]
fn definitions_and_non_strict_link_only_lines_are_exempt() {
    let text = "[long label](https://example.com/long)\n\n![long label](https://example.com/long)\n\n[ref]: https://example.com/long\n\n[long label][ref]\n\n[short](url) [short](url)\n\n`long code value`\n";
    assert_eq!(
        lines(text, json!({"line_length":10,"stern":true})),
        vec![9, 11]
    );
    assert_eq!(
        lines(text, json!({"line_length":10,"strict":true})),
        vec![1, 3, 7, 9, 11]
    );
    assert_eq!(
        lines(
            "[ref]: /url\n    \"a very long title\"\n",
            json!({"line_length":10,"strict":true})
        ),
        Vec::<usize>::new()
    );
}

#[test]
fn only_direct_paragraph_data_defeats_the_link_only_exemption() {
    for text in [
        "[link](url)*alpha beta gamma*",
        "[link](url)**alpha beta gamma**",
        "![image](url)*alpha beta gamma*",
    ] {
        for options in [
            json!({"line_length":10}),
            json!({"line_length":10,"stern":true}),
        ] {
            assert!(findings(text, options).is_empty(), "{text:?}");
        }
        assert_eq!(
            findings(text, json!({"line_length":10,"strict":true})).len(),
            1
        );
    }
    assert_eq!(
        findings(
            "[link](url) *alpha beta gamma*",
            json!({"line_length":10,"stern":true})
        )
        .len(),
        1
    );
}

#[test]
fn bom_and_front_matter_are_not_included_in_markdown_widths() {
    let text = "\u{feff}---\ntitle: a very long value\n---\n\nalpha beta gamma\n";
    assert_eq!(
        lines(text, json!({"line_length":10,"strict":true})),
        vec![5]
    );
    assert_eq!(
        lines(
            "\u{feff}alpha beta",
            json!({"line_length":10,"strict":true})
        ),
        Vec::<usize>::new()
    );
}

#[test]
fn re2_tail_whitespace_and_javascript_stern_whitespace_stay_distinct() {
    assert_eq!(
        lines("ab\u{a0}verylongtoken", json!({"line_length":5})),
        Vec::<usize>::new()
    );
    assert_eq!(
        lines("ab\u{b}verylongtoken", json!({"line_length":5})),
        Vec::<usize>::new()
    );
    assert_eq!(
        lines("ab\tverylongtoken", json!({"line_length":5})),
        Vec::<usize>::new()
    );
    assert_eq!(
        lines(
            "ab\u{a0}verylongtoken",
            json!({"line_length":5,"stern":true})
        ),
        vec![1]
    );
    assert_eq!(
        lines(
            ">\u{a0}verylongtoken",
            json!({"line_length":5,"stern":true})
        ),
        Vec::<usize>::new()
    );
    assert_eq!(lines("alpha beta  ", json!({"line_length":10})), vec![1]);
}

#[test]
fn findings_respect_directives_severity_and_disabled_fix_permissions() {
    let text =
        "<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n\nalpha beta gamma\n";
    let result = findings(
        text,
        json!({"line_length":10,"strict":true,"severity":"warning"}),
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].severity, Severity::Warning);
    assert_eq!(
        Source::new(text.into())
            .position(result[1].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        4
    );
    let cfg = ResolvedConfig {
        fix: false,
        rules: RuleConfig::from_value(json!({"MD013":{"line_length":10,"strict":true}})).unwrap(),
        ..Default::default()
    };
    let result = format_with_config("alpha beta gamma", &FormatOptions::default(), &cfg).unwrap();
    assert_eq!(result.output, "alpha beta gamma");
    assert_eq!(result.diagnostics.len(), 1);
    assert!(findings("alpha beta gamma", json!(false)).is_empty());
}

#[test]
fn output_spans_use_utf8_boundaries_and_utf16_lengths() {
    let result = findings("a😀b\r\n", json!({"line_length":2,"strict":true}));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].span, Some(1..6));
    assert!(result[0].message.contains("Expected: 2; Actual: 4"));
    let cfg = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"MD013":{"line_length":10,"strict":true}})).unwrap(),
        ..Default::default()
    };
    let result =
        format_with_config("alpha verylongtoken tail", &FormatOptions::default(), &cfg).unwrap();
    assert_eq!(result.output, "alpha\nverylongtoken\ntail");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(result.output)
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        2
    );
}

#[test]
fn diagnostic_semantics_match_the_recorded_pinned_oracle() {
    let observations: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md013-observations.json"
    ))
    .unwrap();
    for row in observations.as_array().unwrap() {
        let text = row["input"].as_str().unwrap();
        let source = Source::new(text.into());
        let actual = findings(text, row["options"].clone()).iter().map(|finding| {
            let position = source.position(finding.span.as_ref().unwrap().start).unwrap();
            json!({
                "line":position.line,
                "column":position.column_utf16 - usize::from(position.line == 1 && text.starts_with('\u{feff}')),
                "detail":finding.message.strip_prefix("Line length [").unwrap().strip_suffix(']').unwrap(),
                "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}
            })
        }).collect::<Vec<_>>();
        assert_eq!(
            json!(actual),
            row["findings"],
            "{} input {:?} options {}",
            row["id"],
            text,
            row["options"]
        );
    }
}
