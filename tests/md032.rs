use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD032":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn outer_list_separators_preserve_tightness_and_settle_once() {
    for (input, expected, last) in [
        (
            "before\n- item\n# After\n",
            "before\n\n- item\n\n# After\n",
            2,
        ),
        (
            "before\n- outer\n  - inner\n# After\n",
            "before\n\n- outer\n  - inner\n\n# After\n",
            3,
        ),
    ] {
        let settings = config(json!(true));
        let raw = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
        assert_eq!(raw.output, input);
        assert_eq!(raw.diagnostics.len(), 2);
        let source = Source::new(input.into());
        assert_eq!(
            raw.diagnostics
                .iter()
                .map(|d| source
                    .position(d.span.as_ref().unwrap().start)
                    .unwrap()
                    .line)
                .collect::<Vec<_>>(),
            [2, last]
        );
        let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(first.output, expected);
        assert!(first.diagnostics.is_empty());
        assert!(
            first
                .edits
                .iter()
                .all(|e| e.span.is_empty() && e.origin == "MD032")
        );
        let again =
            format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, first.output);
        assert!(again.edits.is_empty());
    }
}

#[test]
fn outermost_lists_and_visual_ends_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md032-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD032");let span=d.span.clone().unwrap();assert!(span.is_empty());let p=source.position(span.start).unwrap();assert_eq!(p.column_utf16,1+usize::from(p.line==1&&input.starts_with('\u{feff}')));json!({"line":p.line,"message":d.message,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let expected:Vec<_>=row["findings"].as_array().unwrap().iter().map(|f|json!({"line":f["line"],"message":format!("Lists should be surrounded by blank lines [Context: \"{}\"]",f["context"].as_str().unwrap()),"severity":f["severity"]})).collect();
        if actual != expected {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {expected:?}",
                row["id"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn trailing_quote_prefixes_do_not_create_false_visual_end_findings() {
    for input in ["> - item\n>\n# After\n", "- > item\n  >\n# After\n"] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
        assert!(result.edits.is_empty());
    }
    let input = "- *item\n  *\n# After\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        Source::new(input.into())
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        2
    );
}

#[test]
fn lazy_html_keeps_surviving_list_and_quote_ownership_without_source_edits() {
    for (input, lines) in [
        ("- item\n<span>\n- next\n# After\n", vec![3]),
        ("> - item\n<span>\n- next\n# After\n", vec![2, 3, 3]),
        ("- > item\n<span>\n  - next\n# After\n", vec![3]),
        ("- item\n<span>\nlazy text\n# After\n", vec![2]),
        (
            "- item\n<span title='😀'>\n</span>\n- next\n# After\n",
            vec![2],
        ),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| source
                    .position(d.span.as_ref().unwrap().start)
                    .unwrap()
                    .line)
                .collect::<Vec<_>>(),
            lines
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| !d.message.contains("xspan"))
        );
    }
}

#[test]
fn active_directive_targets_keep_original_and_final_edit_coordinates() {
    let input = "before\n- item <!-- markdownlint-disable-next-line MD026 -->\n# After!\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD032":true,"MD026":true})).unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(
        first.output,
        "before\n\n- item <!-- markdownlint-disable-next-line MD026 -->\n# After!\n"
    );
    assert_eq!(first.edits.len(), 1);
    assert_eq!(first.edits[0].span, 7..7);
    assert_eq!(first.edits[0].origin, "MD032");
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD032");
    assert_eq!(
        Source::new(first.output.clone())
            .position(first.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        3
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert_eq!(again.diagnostics, first.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn cli_retains_warning_findings_when_fixes_are_disabled() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "before\n- item\n# After\n";
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"fix":false,"config":{"default":false,"blanks-around-lists":{"severity":"warning"}}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr)
                .matches("Warning MD032")
                .count(),
            2
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}
