use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD014":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn command_only_blocks_report_prompts_without_deleting_code() {
    let input = "```sh\n$ echo hi\n$ pwd\n```\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert_eq!(first.diagnostics.len(), 2);
    assert_eq!(first.diagnostics[0].span, Some(6..8));
    assert_eq!(first.diagnostics[1].span, Some(16..18));
}

#[test]
fn whole_block_classification_and_source_ranges_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md014-observations.json"
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
            assert_eq!(finding.code,"MD014");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"context":finding.message.strip_prefix("Dollar signs used before commands without showing output [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
                "severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
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
fn recovered_root_code_after_generic_html_keeps_prompt_findings() {
    let input = "- item\n<span>\n```sh\n$ echo hi\n```\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(
        Source::new(input.into())
            .position(first.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        4
    );
}

#[test]
fn permissions_warnings_and_reflow_leave_code_and_second_pass_unchanged() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"commands-show-output":{"severity":"warning"},"MD013":{"line_length":20,"code_blocks":false}})).unwrap(),..Default::default()};
    let input = "a paragraph with words that needs wrapping\n\n```sh\n$ echo hi\n```\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(first.output.contains("```sh\n$ echo hi\n```"));
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].severity, Severity::Warning);
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn cli_write_retains_code_and_reports_remaining_prompt_findings() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD014":true}}"#,
    )
    .unwrap();
    let input = "```sh\r\n$ echo hi\r\n```\r\n";
    fs::write(&path, input).unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD014"));
    }
}

#[test]
fn tab_only_fenced_values_still_suppress_whole_block_findings() {
    for input in [
        " ```\n$ hi\n\t\n ```\n",
        "> ```\n> $ hi\n>\t\n> ```\n",
        " ```\n$ hi\n\t",
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

#[test]
fn recovered_fences_continue_through_blank_lines_to_the_actual_end() {
    for (input, expected) in [
        ("- item\n<span>\n```\n$ two\n\noutput\n```\n", 0),
        ("- item\n<span>\n```\n$ two\n\n$ two\n```\n", 2),
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(result.diagnostics.len(), expected, "{input:?}");
        assert_eq!(result.output, input);
    }
}

#[test]
fn indented_code_fences_still_owned_by_html_do_not_become_shell_blocks() {
    for input in [
        "- item\n<span>\n  ```\n  $ two\n  ```\n",
        "\u{feff}- item\n<span>\n  ```\n  $ two\n  ```\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }
}
