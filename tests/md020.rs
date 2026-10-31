use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD020":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn closed_separator_findings_do_not_remove_literal_hashes_or_create_headings() {
    for input in [
        "#Heading#\n",
        "# Heading#\n",
        "#Heading #\n",
        "# Heading\\##\n",
    ] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert!(first.edits.is_empty());
        assert_eq!(first.diagnostics.len(), 1);
        assert_eq!(first.diagnostics[0].code, "MD020");
    }
}

#[test]
fn source_matching_ranges_and_side_specific_context_follow_pinned_cases() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md020-observations.json"
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
            assert_eq!(finding.code,"MD020");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            let astral=input.starts_with("#😀")||input.starts_with("# 😀");
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}'))+usize::from(input.starts_with("# 😀")),
                "length":mdfmt::utf16_len(&input[span])-usize::from(astral),
                "context":finding.message.strip_prefix("No space inside hashes on closed atx style heading [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
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
fn scalar_ranges_are_valid_on_both_sides_of_astral_content() {
    for (input, span) in [
        ("#😀#\n", 0..5),
        ("# 😀#\n", 2..7),
        ("\u{feff}#😀#\r\n", 3..8),
    ] {
        let first = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"severity":"warning"})),
        )
        .unwrap();
        assert_eq!(first.diagnostics.len(), 1);
        assert_eq!(first.diagnostics[0].span, Some(span));
        assert_eq!(first.diagnostics[0].severity, Severity::Warning);
    }
}

#[test]
fn closed_diagnostics_compose_with_approved_spacing_and_line_permissions() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD018":true,"MD020":true,"MD021":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input =
        "#Heading#\n\n#  Good  #\n\n<!-- markdownlint-disable-next-line MD020 -->\n#Disabled#\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(
        first.output,
        "#Heading#\n\n# Good #\n\n<!-- markdownlint-disable-next-line MD020 -->\n#Disabled#\n"
    );
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD020");
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &settings)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn cli_write_reports_preserved_closed_heading_text() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"no-missing-space-closed-atx":true}}"#,
    )
    .unwrap();
    fs::write(&path, "# Heading#\r\n").unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Heading#\r\n");
        assert!(String::from_utf8_lossy(&result.stderr).contains("MD020"));
    }
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"default":false,"MD020":{"severity":"warning"}}}"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Warning MD020"));
}

#[test]
fn reflow_reports_missing_separators_on_the_final_physical_lines() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD018":true,"MD020":true,"MD013":{"line_length":12}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(
        "#closed alpha beta gamma#\n",
        &FormatOptions::default(),
        &settings,
    )
    .unwrap();
    assert_eq!(first.output, "#closed\nalpha beta\ngamma#\n");
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD018");
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn generic_html_after_a_container_does_not_hide_later_root_lines() {
    for input in ["- item\n<span>\n#Heading#\n", "> text\n<span>\n#Heading#\n"] {
        let first =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(first.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            Source::new(input.into())
                .position(first.diagnostics[0].span.as_ref().unwrap().start)
                .unwrap()
                .line,
            3
        );
    }
}
