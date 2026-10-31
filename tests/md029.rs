use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD029":options})).unwrap(),
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
fn later_markers_follow_configured_styles_without_changing_rendered_starts() {
    for (input, expected, options) in [
        (
            "1. one\n3. two\n5. three\n",
            "1. one\n2. two\n3. three\n",
            json!(true),
        ),
        (
            "1. one\n3. two\n5. three\n",
            "1. one\n1. two\n1. three\n",
            json!({"style":"one"}),
        ),
        (
            "0. one\n2. two\n4. three\n",
            "0. one\n0. two\n0. three\n",
            json!({"style":"zero"}),
        ),
        (
            "1. one\n3. two\n5. three\n",
            "1. one\n0. two\n0. three\n",
            json!({"style":"zero"}),
        ),
        (
            "2. one\n4. two\n6. three\n",
            "2. one\n2. two\n3. three\n",
            json!({"style":"ordered"}),
        ),
        (
            "2. one\n1. two\n3. three\n",
            "2. one\n1. two\n1. three\n",
            json!(true),
        ),
        (
            "01. one\n03. two\n05. three\n",
            "01. one\n02. two\n03. three\n",
            json!(true),
        ),
        (
            " 9. one\n10. two\n12. three\n",
            " 9. one\n 2. two\n 3. three\n",
            json!(true),
        ),
        (
            "1. one\n3. \n5. three\n",
            "1. one\n2. \n3. three\n",
            json!(true),
        ),
    ] {
        fixed(input, expected, options);
    }
    let first = format_with_config(
        "2. one\n4. two\n",
        &FormatOptions::default(),
        &config(json!({"style":"ordered"})),
    )
    .unwrap();
    assert_eq!(first.diagnostics.len(), 1);
    assert!(
        first.diagnostics[0]
            .message
            .contains("Expected: 1; Actual: 2")
    );
}

#[test]
fn ordered_prefix_diagnostics_match_pinned_inference_alignment_and_ranges() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md029-observations.json"
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
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD029");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Ordered list item prefix [").unwrap().strip_suffix(']').unwrap(),
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
fn literal_code_and_uncertain_tabbed_marker_boundaries_are_not_reinterpreted() {
    for input in [
        "1. one\n\n10. two\n\n        literal\n",
        "1.\tone\n3.\ttwo\n",
        ">\t1. one\n>\t3. two\n",
    ] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert!(!first.diagnostics.is_empty());
    }
}

#[test]
fn disabled_reference_markers_still_determine_inference_and_expected_positions() {
    fixed(
        "<!-- markdownlint-disable-next-line MD029 -->\n2. one\n4. two\n",
        "<!-- markdownlint-disable-next-line MD029 -->\n2. one\n2. two\n",
        json!(true),
    );
    fixed(
        "1. one\n   <!-- markdownlint-disable-next-line MD029 -->\n3. two\n5. three\n",
        "1. one\n   <!-- markdownlint-disable-next-line MD029 -->\n3. two\n3. three\n",
        json!(true),
    );
}

#[test]
fn numbering_compounds_with_quote_heading_fence_and_eof_stages_in_original_coordinates() {
    let config = ResolvedConfig {rules:RuleConfig::from_value(json!({"default":false,"MD029":true,"MD027":true,"MD023":true,"MD048":{"style":"backtick"},"MD047":true})).unwrap(), ..Default::default()};
    let input = "> 1. one\n> 3. >  text\n>    >   more\n\n~~~js\ncode\n~~~";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(
        result.output,
        "> 1. one\n> 2. > text\n>    > more\n\n```js\ncode\n```\n"
    );
    assert!(result.diagnostics.is_empty());
    let mut applied = String::new();
    let mut cursor = 0;
    for edit in &result.edits {
        applied.push_str(&input[cursor..edit.span.start]);
        applied.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    applied.push_str(&input[cursor..]);
    assert_eq!(applied, result.output);
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn permissions_and_active_line_overrides_survive_prose_reflow() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"ol-prefix":{"style":"ordered","severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = "1. one\n3. two\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD029":true,"MD013":{"line_length":12}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input =
        "<!-- markdownlint-disable-next-line MD029 -->\n2. alpha beta gamma\n4. delta epsilon zeta";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(
        first
            .output
            .lines()
            .filter(|line| line.starts_with("2. "))
            .count(),
        2
    );
    assert_eq!(
        first
            .diagnostics
            .iter()
            .map(|finding| finding.code.as_str())
            .collect::<Vec<_>>(),
        vec!["MD013"]
    );
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_repairs_later_markers_and_reports_preserved_start_mismatches() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    fs::write(&path, "1. one\n3. two\n").unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"ol-prefix":{"style":"ordered"}}}"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), "1. one\n3. two\n");
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "1. one\n2. two\n");
    fs::write(&path, "2. one\n4. two\n").unwrap();
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), "2. one\n2. two\n");
    assert!(String::from_utf8_lossy(&write.stderr).contains("MD029"));
    fs::write(&configuration, r#"{"fix":false,"config":{"default":false,"MD029":{"style":"ordered","severity":"warning"}}}"#).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"2. one\n2. two\n");
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD029"));
}

#[test]
fn empty_items_do_not_turn_paragraph_continuations_into_ordered_lists() {
    for input in [
        "\u{feff}009. \r\n999999999.item1\r\n10.    item2\r\n01.\r\n2.    \r\n",
        "9.\ntext\n2. item\n4. item\n",
        "> 9.\n> text\n> 2. item\n> 4. item\n",
    ] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert_eq!(
            first.diagnostics.len(),
            1,
            "{input:?}: {:?}",
            first.diagnostics
        );
        assert_eq!(
            Source::new(input.into())
                .position(first.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            1
        );
    }
}

#[test]
fn ambiguous_empty_item_boundaries_block_unrelated_edits() {
    let input = "# Heading!\n\n9.\ntext\n2. item\n4. item\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD026":true,"MD029":true})).unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert!(
        first
            .diagnostics
            .iter()
            .any(|finding| finding.code == "MD026")
    );
}

#[test]
fn interruption_state_survives_indented_code_and_new_quote_boundaries() {
    for (input, expected_lines) in [
        ("    code\n2. a\n4. b\n1. c\n", vec![]),
        ("9.\ntext\n> 2. a\n> 4. b\n", vec![1]),
        ("text\n> 2. a\n> 4. b\n> 1. c\n", vec![]),
        ("    code\n> 2. a\n> 4. b\n> 1. c\n", vec![]),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input, "{input:?}");
        let source = Source::new(input.into());
        let lines: Vec<_> = result
            .diagnostics
            .iter()
            .map(|finding| {
                source
                    .position(finding.span.as_ref().unwrap().start)
                    .unwrap()
                    .line
            })
            .collect();
        assert_eq!(lines, expected_lines, "{input:?}");
    }
}
