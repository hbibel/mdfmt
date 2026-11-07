use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD060":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn styles_report_literal_pipe_positions_without_rewriting_tables() {
    let input = "| a |\n| --- |\n| b |\n";
    for (style, count) in [("aligned", 1), ("compact", 0), ("tight", 6), ("any", 0)] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"style":style})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), count, "{style}");
        if style == "aligned" {
            assert_eq!(result.diagnostics[0].span, Some(12..13));
        }
    }
}

#[test]
fn display_alignment_padding_and_style_selection_match_pinned_ranges() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md060-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input, "{}", row["id"]);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code,"MD060");
            let span=finding.span.clone().unwrap();
            assert_eq!(&input[span.clone()],"|");
            let position=source.position(span.start).unwrap();
            json!({"line":position.line,"range":[position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),1],"detail":finding.message.strip_prefix("Table column style [").unwrap().strip_suffix(']').unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        if actual != *row["findings"].as_array().unwrap() {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn permissions_and_final_positions_survive_prose_reflow() {
    let input = "a paragraph with several words to reflow\n\n| a |\n| --- |\n| b |\n";
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"table-column-style":{"style":"aligned","severity":"warning"},"MD013":{"line_length":20,"tables":false}})).unwrap(),..Default::default()};
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(
        first.output,
        "a paragraph with\nseveral words to\nreflow\n\n| a |\n| --- |\n| b |\n"
    );
    assert_eq!(first.diagnostics.len(), 1);
    let source = Source::new(first.output.clone());
    assert_eq!(
        source
            .position(first.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        6
    );
    assert_eq!(first.diagnostics[0].severity, Severity::Warning);
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert_eq!(again.diagnostics, first.diagnostics);
    let input = "| a |\n| - |\n| bb | <!-- markdownlint-disable-line MD060 -->\n| ccc |\n";
    let result = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"style":"aligned"})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(input.into())
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        4
    );
}

#[test]
fn cli_reports_table_errors_and_warnings_without_changing_files() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "| a |\n| --- |\n| b |\n";
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD060":{"style":"aligned"}}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Error MD060"));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"fix":false,"config":{"default":false,"MD060":{"style":"aligned","severity":"warning"}}}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, input.as_bytes());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning MD060"));
}
