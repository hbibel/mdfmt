use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD047":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn final_line_diagnostics_match_recorded_upstream_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md047-observations.json"
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
        let actual: Vec<_> = result.diagnostics.iter().map(|diagnostic| {
            assert_eq!(diagnostic.code, "MD047");
            let position = source.position(diagnostic.span.as_ref().unwrap().start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "severity":if diagnostic.severity == Severity::Warning { "warning" } else { "error" }})
        }).collect();
        let expected: Vec<_> = row["findings"].as_array().unwrap().iter().map(|finding| {
            json!({"line":finding["line"],"column":finding["column"].as_u64().unwrap() - u64::from(input.chars().next_back().is_some_and(|ch| ch.len_utf16() == 2)), "severity":finding["severity"]})
        }).collect();
        assert_eq!(actual, expected, "{}", row["id"]);
    }
}

#[test]
fn safe_final_newlines_use_the_local_terminator_and_are_idempotent() {
    for (input, expected) in [
        ("hello", "hello\n"),
        ("one\r\ntwo", "one\r\ntwo\r\n"),
        ("one\rtwo", "one\rtwo\r"),
        ("\u{feff}hello", "\u{feff}hello\n"),
        ("# Title", "# Title\n"),
        ("[link](url)", "[link](url)\n"),
        ("`code`", "`code`\n"),
        ("hello ", "hello \n"),
        ("hello <!-- comment -->", "hello <!-- comment -->\n"),
        ("- hello", "- hello\n"),
        ("> hello", "> hello\n"),
        ("```\ncode\n```", "```\ncode\n```\n"),
    ] {
        let config = config(json!(true));
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(result.output, expected, "{input:?}");
        assert!(result.diagnostics.is_empty());
        let second =
            format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, result.output);
        assert!(second.edits.is_empty());
    }
}

#[test]
fn upstream_blank_line_exceptions_do_not_require_a_newline() {
    for input in [
        "",
        " ",
        "\t",
        "\u{a0}",
        "\u{feff}",
        ">",
        "> >",
        "<!-- comment -->",
        "<!-- comment",
        "tail -->",
        "<!-- one --><!-- two -->",
        "hello\n>",
        "hello\n\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input, "{input:?}");
        assert!(result.diagnostics.is_empty(), "{input:?}");
    }
}

#[test]
fn literal_eof_boundaries_are_preserved_with_remaining_findings() {
    for input in [
        "```\ncode",
        "```\n> ```",
        "> ```\n> > ```",
        "```\n    ```",
        "    ```\n    ```",
        "    code",
        "<div>\nhello",
        "| a |\n| - |\n| b |",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input, "{input:?}");
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 1, "{input:?}");
        assert_eq!(result.diagnostics[0].code, "MD047");
    }
}

#[test]
fn newline_diagnostics_use_byte_safe_spans_and_configured_permissions() {
    let input = "text 😀";
    let config = config(json!({"severity":"warning"}));
    let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    let finding = &result.diagnostics[0];
    assert_eq!(finding.severity, Severity::Warning);
    let span = finding.span.clone().unwrap();
    assert_eq!(&input[span.clone()], "😀");
    assert_eq!(
        Source::new(input.into())
            .position(span.start)
            .unwrap()
            .column_utf16,
        6
    );
    let mut config = config;
    config.fix = false;
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .diagnostics
            .len(),
        1
    );
    for input in [
        "<!-- markdownlint-disable-file MD047 -->\nhello",
        "<!-- markdownlint-disable-next-line MD047 -->\nhello",
    ] {
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn combined_rules_can_edit_headings_reflow_and_insert_at_eof() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD013":{"line_length":10},"MD026":true,"MD047":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(
        "# Title!\n\nalpha\nbeta gamma",
        &FormatOptions::default(),
        &config,
    )
    .unwrap();
    assert_eq!(result.output, "# Title\n\nalpha beta\ngamma\n");
    assert!(result.diagnostics.is_empty());
    let result = format_with_config("# Title!", &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "# Title\n");
    let mut output = String::new();
    let mut cursor = 0;
    for edit in &result.edits {
        output.push_str(&"# Title!"[cursor..edit.span.start]);
        output.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    output.push_str(&"# Title!"[cursor..]);
    assert_eq!(output, result.output);
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}
