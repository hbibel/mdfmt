use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD056":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn table_cell_mismatches_are_reported_without_adding_or_deleting_cells() {
    for input in [
        "| a | b |\n| - | - |\n| c |\n",
        "| a | b |\n| - | - |\n| c | d | e |\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "MD056");
    }
}

#[test]
fn counts_and_source_ranges_match_pinned_table_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md056-observations.json"
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
            assert_eq!(finding.code,"MD056");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}'))+usize::from(input.ends_with("\n|😀\n")),"length":mdfmt::utf16_len(&input[span])-usize::from(input.ends_with("\n|😀\n")),
                "detail":finding.message.strip_prefix("Table column count [").unwrap().strip_suffix(']').unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
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
fn recovered_tables_keep_root_ownership_after_generic_html() {
    let input = "- item\n<span>\n| a | b |\n| - | - |\n| c |\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(input.into())
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        5
    );
}

#[test]
fn per_table_inference_and_warning_permissions_survive_prose_reflow() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"table-column-count":{"severity":"warning"},"MD013":{"line_length":20,"tables":false}})).unwrap(),..Default::default()};
    let input = "a paragraph with words that needs wrapping\n\n| a | b |\n| - | - |\n| c |\n\n| a |\n| - |\n| b | c |\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(
        first
            .output
            .ends_with("| a | b |\n| - | - |\n| c |\n\n| a |\n| - |\n| b | c |\n")
    );
    assert_eq!(first.diagnostics.len(), 2);
    assert!(
        first
            .diagnostics
            .iter()
            .all(|finding| finding.severity == Severity::Warning)
    );
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn cli_write_preserves_table_source_and_reports_remaining_counts() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD056":true}}"#,
    )
    .unwrap();
    let input = "| a | b |\r\n| - | - |\r\n| c |\r\n";
    fs::write(&path, input).unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD056"));
    }
}

#[test]
fn missing_cell_ranges_never_start_inside_an_astral_scalar() {
    let input = "| a | b |\n| - | - |\n|😀\n";
    let first =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].span, Some(21..25));
    assert_eq!(first.output, input);
}

#[test]
fn nested_quote_recovery_preserves_the_surviving_list_and_paragraph_context() {
    for (input, expected) in [
        ("- > quote\n<span>\n  | a | b |\n  | - | - |\n  | c |\n", 1),
        (
            "- > quote\n<span>\n  owned\n| a | b |\n| - | - |\n| c |\n",
            0,
        ),
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(
            result.diagnostics.len(),
            expected,
            "{input:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.output, input);
    }
}

#[test]
fn an_earlier_lazy_table_fragment_cannot_hide_a_separate_root_table() {
    let input =
        "> quote\n> <span>\n  | x | y |\n  | - | - |\n  | z |\n\n| a | b |\n| - | - |\n| c |\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(input.into())
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        9
    );
}

#[test]
fn nested_quote_markers_are_relative_to_surviving_ordered_list_padding() {
    let input = "10. > quote\n<span>\n    > | a | b |\n    > | - | - |\n    > | c |\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn completed_nonparagraph_containers_do_not_make_following_html_lazy() {
    for input in [
        "- ```\n  code\n  ```\n<span>\n| a | b |\n| - | - |\n| c |\n",
        "- # heading\n<span>\n| a | b |\n| - | - |\n| c |\n",
        "-\n<span>\n| a | b |\n| - | - |\n| c |\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert!(
            result.diagnostics.is_empty(),
            "{input:?}: {:?}",
            result.diagnostics
        );
    }
}
