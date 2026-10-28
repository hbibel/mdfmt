use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD004":options})).unwrap(),
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
fn explicit_and_consistent_markers_preserve_list_structure() {
    fixed("- one\n- two\n", "+ one\n+ two\n", json!({"style":"plus"}));
    fixed("* one\n", "- one\n", json!({"style":"dash"}));
    fixed("- one\n", "* one\n", json!({"style":"asterisk"}));
    fixed(
        "- one\n\nparagraph\n\n+ two\n",
        "- one\n\nparagraph\n\n- two\n",
        json!(true),
    );
    fixed(
        "> - one\n>   - child\n",
        "> + one\n>   + child\n",
        json!({"style":"plus"}),
    );
    fixed(
        "- [ ] todo\n  - [x] done\n",
        "+ [ ] todo\n  + [x] done\n",
        json!({"style":"plus"}),
    );
    fixed(
        "\u{feff}- 😀one\r\n  - child\r\n",
        "\u{feff}+ 😀one\r\n  + child\r\n",
        json!({"style":"plus"}),
    );
}

#[test]
fn sublist_style_counts_ordered_ancestors_and_uses_global_depth_choices() {
    fixed(
        "- root\n  - child\n    + grandchild\n",
        "- root\n  + child\n    * grandchild\n",
        json!({"style":"sublist"}),
    );
    fixed(
        "* root\n  * child\n    * grandchild\n",
        "* root\n  - child\n    * grandchild\n",
        json!({"style":"sublist"}),
    );
    fixed(
        "1. root\n   - child\n     - grandchild\n",
        "1. root\n   - child\n     + grandchild\n",
        json!({"style":"sublist"}),
    );
}

#[test]
fn unsafe_list_merges_or_thematic_breaks_are_not_forced() {
    for (input, options) in [
        ("* one\n+ two\n", json!(true)),
        ("- * *\n", json!({"style":"asterisk"})),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(options)).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert!(!result.diagnostics.is_empty());
    }
}

#[test]
fn disabled_reference_markers_still_set_expected_style() {
    let input = "<!-- markdownlint-disable-next-line MD004 -->\n* one\n\nparagraph\n\n- two\n";
    fixed(
        input,
        "<!-- markdownlint-disable-next-line MD004 -->\n* one\n\nparagraph\n\n* two\n",
        json!(true),
    );
    let input = "<!-- markdownlint-disable-file bullet -->\n* one\n\nparagraph\n\n- two\n";
    fixed(input, input, json!(true));
    fixed("- one\n", "- one\n", json!(false));
}

#[test]
fn diagnostics_match_recorded_pinned_style_and_nesting_semantics() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md004-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        if row["options"]["style"] == "invalid" {
            assert!(RuleConfig::from_value(json!({"MD004":row["options"]})).is_err());
            continue;
        }
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        let source = Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            let position = source.position(finding.span.as_ref().unwrap().start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "detail":finding.message.strip_prefix("Unordered list style [").unwrap().strip_suffix(']').unwrap(),
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
fn all_rule_edits_keep_original_coordinates_and_stable_output() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD004":{"style":"plus"},"MD012":true,"MD013":{"line_length":10},"MD019":true,"MD021":true,"MD026":true,"MD035":{"style":"* * *"},"MD047":true})).unwrap(), ..Default::default() };
    let input = "#  Title!\n\n\n- alpha beta gamma\n\n---";
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

#[test]
fn thematic_style_repairs_can_unlock_list_marker_repairs_in_the_same_invocation() {
    for (marker, thematic, expected) in [("asterisk", "___", "* ___\n"), ("plus", "---", "+ ---\n")]
    {
        let config = ResolvedConfig {
            rules: RuleConfig::from_value(
                json!({"default":false,"MD004":{"style":marker},"MD035":{"style":thematic}}),
            )
            .unwrap(),
            ..Default::default()
        };
        let input = "- ***\n";
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected);
        assert!(first.diagnostics.is_empty());
        let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, first.output);
        assert!(second.edits.is_empty());
        let mut applied = String::new();
        let mut cursor = 0;
        for edit in &first.edits {
            applied.push_str(&input[cursor..edit.span.start]);
            applied.push_str(&edit.replacement);
            cursor = edit.span.end;
        }
        applied.push_str(&input[cursor..]);
        assert_eq!(applied, first.output);
    }
}

#[test]
fn tabbed_container_prefixes_do_not_expose_indented_code_as_list_markers() {
    let input = "> 1.\t---\n >  \t+\n";
    let config = config(json!({"style":"asterisk"}));
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert!(first.diagnostics.is_empty());
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn quote_indentation_does_not_change_container_relative_sublist_depth() {
    let input = " > - root\n>   - child\n";
    let config = config(json!({"style":"sublist"}));
    let unchecked = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
    assert_eq!(unchecked.diagnostics.len(), 1);
    let position = Source::new(input.into())
        .position(unchecked.diagnostics[0].span.as_ref().unwrap().start)
        .unwrap();
    assert_eq!((position.line, position.column_utf16), (2, 5));
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, " > - root\n>   + child\n");
    assert!(first.diagnostics.is_empty());
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn uncertain_tabbed_container_boundaries_also_block_other_rule_edits() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD019":true,"MD021":true,"MD026":true,"MD035":{"style":"***"},"MD047":true})).unwrap(), ..Default::default() };
    let input = "> 1.\t---\n >  \t#  Hello!";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}
