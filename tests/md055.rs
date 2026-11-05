use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD055":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn pipe_style_findings_preserve_every_table_byte() {
    let input = "| a | b |\n| - | - |\nc | d\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|finding| finding.code == "MD055")
    );
}

#[test]
fn inference_delimiter_rows_and_ranges_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md055-observations.json"
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
            assert_eq!(finding.code,"MD055");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();let scalar=mdfmt::utf16_len(&input[span.clone()])==2&&input[span.clone()].chars().count()==1;
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}'))+usize::from(scalar&&finding.message.ends_with("trailing pipe]")),"length":mdfmt::utf16_len(&input[span])-usize::from(scalar),
                "detail":finding.message.strip_prefix("Table pipe style [").unwrap().strip_suffix(']').unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
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
fn disabled_header_rows_still_determine_document_and_per_table_style() {
    let input = "<!-- markdownlint-disable-next-line MD055 -->\n| a | b |\n| - | - |\nc | d\n\na | b\n--- | ---\nc | d\n";
    for (options, expected) in [(json!(true), 8), (json!({"style":"per_table"}), 2)] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(options)).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), expected);
    }
}

#[test]
fn delimiter_indentation_is_not_confused_with_container_padding() {
    let input = "  | a | b |  \n  | - | - |  \n  c | d  \n";
    let first = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"style":"no_leading_or_trailing"})),
    )
    .unwrap();
    assert_eq!(first.diagnostics.len(), 4);
    assert_eq!(first.diagnostics[2].span, Some(14..15));
}

#[test]
fn scalar_edge_ranges_are_byte_valid_and_warning_reflow_is_stable() {
    let input = "| a | b |\n| - | - |\n😀 | d\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.diagnostics[0].span, Some(20..24));
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"table-pipe-style":{"severity":"warning"},"MD013":{"line_length":20,"tables":false}})).unwrap(),..Default::default()};
    let input = format!("a paragraph with many words to wrap\n\n{input}");
    let first = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
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
fn cli_write_preserves_pipe_style_and_reports_remaining_findings() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD055":true}}"#,
    )
    .unwrap();
    let input = "| a | b |\r\n| - | - |\r\nc | d\r\n";
    fs::write(&path, input).unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD055"));
    }
}

#[test]
fn continuation_and_inline_code_body_rows_do_not_disappear_from_style_checks() {
    for (first, next) in [("", ""), ("> ", "> "), ("- ", "  ")] {
        for body in [
            format!("{next}| c | d |\n{next}e | f\n"),
            format!("{next}`c | d`\n"),
        ] {
            let input = format!("{first}| a | b |\n{next}| --- | --- |\n{body}");
            let result =
                format_with_config(&input, &FormatOptions { fix: false }, &config(json!(true)))
                    .unwrap();
            assert_eq!(result.output, input);
            assert_eq!(
                result.diagnostics.len(),
                2,
                "{input:?}: {:?}",
                result.diagnostics
            );
        }
    }
}

#[test]
fn diagnostic_table_recovery_also_preserves_rows_during_prose_reflow() {
    let table = "| a | b |\n| --- | --- |\n| c | d |\necho alpha beta gamma | delta\n";
    let input = format!("# Heading!\n\n{table}");
    let settings=ResolvedConfig {rules:RuleConfig::from_value(json!({"default":false,"MD055":true,"MD026":true,"MD013":{"line_length":12,"tables":false}})).unwrap(),..Default::default()};
    let result = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, format!("# Heading\n\n{table}"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|finding| finding.code == "MD055")
            .count(),
        2
    );
    assert!(
        result
            .diagnostics
            .iter()
            .all(|finding| finding.code == "MD055")
    );
}
