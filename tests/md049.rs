use mdfmt::{
    FormatOptions, FormatResult, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD049":options})).unwrap(),
        ..Default::default()
    }
}

fn stable(input: &str, expected: &str, settings: &ResolvedConfig) -> FormatResult {
    let first = format_with_config(input, &FormatOptions::default(), settings).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let mut applied = input.to_owned();
    for edit in first.edits.iter().rev() {
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(applied, first.output);
    let second = format_with_config(&first.output, &FormatOptions::default(), settings).unwrap();
    assert_eq!(second.output, first.output);
    assert_eq!(second.diagnostics, first.diagnostics);
    assert!(second.edits.is_empty());
    first
}

#[test]
fn emphasis_pairs_change_only_markers_and_settle_once() {
    let input = "_one_ and *two*\n";
    let settings = config(json!({"style":"asterisk"}));
    let raw = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(raw.output, input);
    assert_eq!(raw.diagnostics.len(), 2);
    assert_eq!(raw.diagnostics[0].span, Some(0..1));
    assert_eq!(raw.diagnostics[1].span, Some(4..5));
    assert!(raw.diagnostics.iter().all(|d| d.code == "MD049"
        && d.message == "Emphasis style [Expected: asterisk; Actual: underscore]"));
    let first = stable(input, "*one* and *two*\n", &settings);
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.edits.len(), 2);
    assert!(
        first
            .edits
            .iter()
            .all(|e| e.origin == "MD049" && e.replacement == "*")
    );
}

#[test]
fn emphasis_diagnostics_match_pinned_ranges_styles_and_permissions() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md049-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let source = Source::new(input.into());
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let actual: Vec<_> = result.diagnostics.iter().map(|d| {
            assert_eq!(d.code,"MD049");
            let span = d.span.clone().unwrap();
            let start = source.position(span.start).unwrap();
            let end = source.position(span.end).unwrap();
            assert_eq!(start.line,end.line);
            json!({"line":start.line,"column":start.column_utf16-usize::from(start.line==1&&input.starts_with('\u{feff}')),"length":end.column_utf16-start.column_utf16,
                "detail":d.message.strip_prefix("Emphasis style [").unwrap().strip_suffix(']').unwrap(),
                "severity":if d.severity==Severity::Warning {"warning"} else {"error"}})
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
fn links_tables_code_and_metadata_do_not_grant_marker_edit_authority() {
    let settings = config(json!({"style":"asterisk"}));
    for (input, count) in [
        ("[a _b_](url)\n", 2),
        ("![a _b_](img)\n", 2),
        ("| _one_ |\n| --- |\n", 2),
        ("`_one_`\n", 0),
        ("```md\n_one_\n```\n", 0),
        ("---\ntitle: _one_\n---\n", 0),
    ] {
        let first = stable(input, input, &settings);
        assert!(first.edits.is_empty());
        assert_eq!(first.diagnostics.len(), count, "{input:?}");
    }
}

#[test]
fn multiline_markers_require_both_permissions_but_report_each_enabled_end() {
    let input = "*first <!-- markdownlint-disable-next-line MD049 -->\nsecond*\n";
    let first = stable(input, input, &config(json!({"style":"underscore"})));
    assert!(first.edits.is_empty());
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].span, Some(0..1));
}

#[test]
fn consistent_style_uses_disabled_first_tokens() {
    let input = "<!-- markdownlint-disable-next-line MD049 -->\n_one_\n\n*two*\n";
    let first = stable(
        input,
        "<!-- markdownlint-disable-next-line MD049 -->\n_one_\n\n_two_\n",
        &config(json!(true)),
    );
    assert!(first.diagnostics.is_empty());
}

#[test]
fn ascii_intraword_exclusions_do_not_hide_unicode_findings_or_allow_unsafe_edits() {
    let settings = config(json!({"style":"underscore"}));
    let first = stable("a*one*b\n", "a*one*b\n", &settings);
    assert!(first.diagnostics.is_empty());
    for input in ["é*one*é\n", "中*one*中\n", "😀*one*😀\n"] {
        let first = stable(input, input, &settings);
        assert_eq!(first.diagnostics.len(), 2);
        assert!(first.edits.is_empty());
    }
}

#[test]
fn astral_adjacent_underscores_are_literal_in_the_pinned_parser() {
    for input in ["😀_one_😀\n", "😀__one__😀\n"] {
        let first = stable(input, input, &config(json!({"style":"asterisk"})));
        assert!(first.diagnostics.is_empty());
        assert!(first.edits.is_empty());
    }
}

#[test]
fn nested_triple_and_multiline_markers_preserve_structure_and_original_positions() {
    let settings = config(json!({"style":"underscore"}));
    for (input, expected) in [
        ("***both***\n", "_**both**_\n"),
        ("*a _b_ c*\n", "_a _b_ c_\n"),
        ("> *first\n> second*\n", "> _first\n> second_\n"),
        ("\u{feff}*one*\r\n", "\u{feff}_one_\r\n"),
    ] {
        let first = stable(input, expected, &settings);
        assert!(first.diagnostics.is_empty());
    }
}

#[test]
fn marker_heavy_input_does_not_reparse_once_per_pair() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    fs::write(&path, format!("{}\n", "_one_ ".repeat(800))).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD049":{"style":"asterisk"}}}"#,
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .args(["--write", "doc.md"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("marker-heavy input exceeded the validation work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        format!("{}\n", "*one* ".repeat(800))
    );
}
