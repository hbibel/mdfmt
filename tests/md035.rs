use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD035":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let config = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output, "{input:?}");
    assert!(second.edits.is_empty());
}

#[test]
fn consistent_and_explicit_styles_are_repaired_contextually() {
    fixed("***\n\n---\n", "***\n\n***\n", json!(true));
    fixed("Intro\n\n***\n", "Intro\n\n---\n", json!({"style":"---"}));
    fixed("  ***\n\n---\n", "  ***\n\n***\n", json!(true));
    fixed("***  \n\n---\n", "***  \n\n***  \n", json!(true));
    fixed(
        "> intro\n>\n> ***\n",
        "> intro\n>\n> ---\n",
        json!({"style":"---"}),
    );
    fixed("- ***\n", "- ___\n", json!({"style":"___"}));
    fixed(
        "\u{feff}***\r\n\r\n---\r\n",
        "\u{feff}***\r\n\r\n***\r\n",
        json!(true),
    );
}

#[test]
fn invalid_styles_or_changed_blocks_are_diagnosed_without_forcing_a_repair() {
    for (input, options) in [
        ("***\n", json!({"style":"# heading"})),
        ("***\n", json!({"style":"   "})),
        ("alpha\n***\n", json!({"style":"---"})),
        ("- ***\n", json!({"style":"---"})),
        ("***\n\n***\n", json!({"style":"---"})),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(options)).unwrap();
        assert_eq!(result.output, input, "{input:?}");
        assert!(result.edits.is_empty());
        assert!(!result.diagnostics.is_empty(), "{input:?}");
    }
}

#[test]
fn disabled_first_break_still_determines_consistent_style() {
    fixed(
        "<!-- markdownlint-disable-next-line MD035 -->\n***\n\n---\n",
        "<!-- markdownlint-disable-next-line MD035 -->\n***\n\n***\n",
        json!(true),
    );
    let input = "<!-- markdownlint-disable-file hr -->\n***\n\n---\n";
    fixed(input, input, json!(true));
    let input = "***\n\n---\n";
    fixed(input, input, json!(false));
}

#[test]
fn protected_nonbreak_source_is_not_rewritten() {
    for input in [
        "alpha\n---\n",
        "---\ntitle: Hi\n---\n",
        "```\n***\n---\n```\n",
        "| *** |\n| --- |\n| --- |\n",
    ] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"style":"___"})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn diagnostics_match_recorded_pinned_raw_style_semantics() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md035-observations.json"
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
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| json!({
            "line":source.position(finding.span.as_ref().unwrap().start).unwrap().line,
            "detail":finding.message.strip_prefix("Horizontal rule style [").unwrap().strip_suffix(']').unwrap(),
            "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}
        })).collect();
        assert_eq!(
            actual,
            *row["findings"].as_array().unwrap(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn style_replacements_compose_with_earlier_source_edits_and_eof() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD012":true,"MD013":{"line_length":10},"MD019":true,"MD021":true,"MD026":true,"MD035":{"style":"* * *"},"MD047":true})).unwrap(), ..Default::default() };
    for (input, expected) in [
        ("***!\n---\n", "* * *\n* * *\n"),
        (
            "#  Title!\n\n***\n\n\nalpha beta gamma\n\n---",
            "# Title\n\n* * *\n\nalpha beta\ngamma\n\n* * *\n",
        ),
        (
            "#  Title!\n\n> alpha beta gamma\n>\n> ---",
            "# Title\n\n> alpha\n> beta\n> gamma\n>\n> * * *\n",
        ),
    ] {
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected);
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
}
