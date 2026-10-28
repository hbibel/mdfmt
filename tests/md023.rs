use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD023":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn heading_prefix_deletions_preserve_container_padding() {
    for (input, expected) in [
        ("  # Heading\n", "# Heading\n"),
        ("  Heading\n  ===\n", "Heading\n  ===\n"),
        (">  # Heading\n", "> # Heading\n"),
        (" > # Heading\n", "> # Heading\n"),
        (" >  # Heading\n", "> # Heading\n"),
        ("-  # Heading\n", "-  # Heading\n"),
        ("- item\n\n    # Heading\n", "- item\n\n  # Heading\n"),
        ("   A\n   B\n---\n", "A\n   B\n---\n"),
        ("\u{feff}  # 😀 Heading\r\n", "\u{feff}# 😀 Heading\r\n"),
    ] {
        let config = config(json!(true));
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected, "{input:?}");
        assert!(
            format_with_config(&first.output, &FormatOptions::default(), &config)
                .unwrap()
                .edits
                .is_empty()
        );
    }
}

#[test]
fn diagnostics_match_pinned_line_prefix_evidence() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md023-observations.json"
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
            assert_eq!(finding.code, "MD023");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":span.len(),"context":finding.message.strip_prefix("Headings must start at the beginning of the line [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
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
fn source_permissions_and_uncertain_literal_boundaries_prevent_edits() {
    for input in [
        "```\n  # Code\n```\n",
        " > 1.\t---\n >  \t# Code\n",
        "<!-- markdownlint-disable-next-line heading-start-left -->\n  # Heading\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
    }
    let mut config = config(json!({"severity":"warning"}));
    let input = "  # Heading\n";
    for fix in [false, true] {
        config.fix = false;
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn heading_and_whitespace_rules_settle_with_original_coordinates() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD009":true,"MD010":true,"MD019":true,"MD021":true,"MD023":true,"MD026":true,"MD047":true})).unwrap(), ..Default::default() };
    let input = "  #  Heading!  ##\n\nalpha\tbeta ";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "# Heading ##\n\nalpha beta\n");
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

#[test]
fn empty_list_markers_use_default_continuation_padding() {
    for (input, column, length, expected) in [
        ("-   \n    # H\n", 3, 2, "-   \n  # H\n"),
        ("-\t\n   # H\n", 3, 1, "-\t\n   # H\n"),
        ("1. \t\n     # H\n", 4, 2, "1. \t\n     # H\n"),
    ] {
        let config = config(json!(true));
        let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
        assert_eq!(result.diagnostics.len(), 1, "{input:?}");
        let span = result.diagnostics[0].span.clone().unwrap();
        let position = Source::new(input.into()).position(span.start).unwrap();
        assert_eq!(
            (position.line, position.column_utf16, span.len()),
            (2, column, length)
        );
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected);
        assert!(
            format_with_config(&first.output, &FormatOptions::default(), &config)
                .unwrap()
                .edits
                .is_empty()
        );
    }
}
