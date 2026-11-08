use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD039":options})).unwrap(),
        ..Default::default()
    }
}

fn apply_edits(input: &str, edits: &[mdfmt::AppliedEdit]) -> Option<String> {
    let mut applied = input.to_owned();
    for edit in edits.iter().rev() {
        applied.get(edit.span.clone())?;
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    Some(applied)
}

#[test]
fn unicode_label_spaces_have_valid_ranges_even_when_upstream_throws() {
    for space in ['\u{a0}', '\u{2003}', '\u{feff}'] {
        let input = format!("[{space}label{space}](u)\n");
        let result =
            format_with_config(&input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            [
                1..1 + space.len_utf8(),
                6 + space.len_utf8()..6 + 2 * space.len_utf8()
            ]
        );
    }
}

#[test]
fn unicode_label_spaces_do_not_move_findings_to_adjacent_lines() {
    for space in ['\u{a0}', '\u{2003}', '\u{feff}'] {
        for (input, span) in [
            (format!("[{space}label](u)\n"), 1..1 + space.len_utf8()),
            (format!("[label{space}](u)\n"), 6..6 + space.len_utf8()),
        ] {
            let result =
                format_with_config(&input, &FormatOptions::default(), &config(json!(true)))
                    .unwrap();
            assert_eq!(result.output, input);
            assert!(result.edits.is_empty());
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(result.diagnostics[0].span, Some(span));
        }
    }
}

#[test]
fn link_label_findings_do_not_trim_indivisible_links() {
    let input = "[ label ](url)\n";
    let settings = config(json!(true));
    for fix in [false, true] {
        let result = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            [1..2, 7..8]
        );
        assert!(result.diagnostics.iter().all(|d| d.code == "MD039"
            && d.message == "Spaces inside link text [Context: \"[ label ]\"]"));
        let again = format_with_config(&result.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(again.output, input);
        assert_eq!(again.diagnostics, result.diagnostics);
        assert!(again.edits.is_empty());
    }
    let image = "![ label ](img)\n";
    let result = format_with_config(image, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, image);
    assert!(result.diagnostics.is_empty());
    let multiline = "[\n label\n](url)\n";
    let result = format_with_config(multiline, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, multiline);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.span.clone().unwrap())
            .collect::<Vec<_>>(),
        [2..2, 2..2]
    );
}

#[test]
fn link_space_ranges_contexts_and_permissions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md039-observations.json"
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
        assert!(result.edits.is_empty());
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD039");let span=d.span.clone().unwrap();let p=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(p.line,end.line);json!({"line":p.line,"range":if span.is_empty(){Value::Null}else{json!([p.column_utf16-usize::from(p.line==1&&input.starts_with('\u{feff}')),end.column_utf16-p.column_utf16])},"context":d.message.strip_prefix("Spaces inside link text [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"detail":Value::Null,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let expected: Vec<_> = row["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let mut f = f.clone();
                f.as_object_mut().unwrap().remove("contextUtf16");
                f
            })
            .collect();
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
fn link_text_stays_indivisible_and_findings_follow_final_reflow() {
    let input = "alpha beta [ label ](url) gamma delta\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD039":true,"MD013":{"line_length":20}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, "alpha beta\n[ label ](url) gamma\ndelta\n");
    assert_eq!(apply_edits(input, &result.edits).unwrap(), result.output);
    assert_eq!(result.diagnostics.len(), 2);
    let source = Source::new(result.output.clone());
    assert!(result.diagnostics.iter().all(|d| {
        source
            .position(d.span.as_ref().unwrap().start)
            .unwrap()
            .line
            == 2
    }));
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn spacing_findings_compose_with_heading_markers_and_separators() {
    let input = "# _one_!\nalpha * beta * omega\n\n[ label ](url) and `  code  `\n";
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD037":true,"MD038":true,"MD039":true,"MD026":true,"MD049":{"style":"asterisk"},"MD050":{"style":"underscore"},"MD022":true})).unwrap(),..Default::default()};
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(
        result.output,
        "# *one*\n\nalpha * beta * omega\n\n[ label ](url) and `  code  `\n"
    );
    assert_eq!(apply_edits(input, &result.edits).unwrap(), result.output);
    for code in ["MD037", "MD038", "MD039"] {
        assert_eq!(
            result.diagnostics.iter().filter(|d| d.code == code).count(),
            2
        );
    }
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn cli_spacing_aliases_respect_severity_and_no_fix_in_check_and_write_modes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "alpha * beta * omega\n\n`  code  ` [ label ](url)\n";
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [true, false] {
            fs::write(directory.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"no-space-in-emphasis":{"severity":severity},"no-space-in-code":{"severity":severity},"no-space-in-links":{"severity":severity}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(directory.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.code(),
                    Some(if severity == "error" { 1 } else { 0 })
                );
                assert!(result.stdout.is_empty());
                let stderr = String::from_utf8_lossy(&result.stderr);
                for rule in ["MD037", "MD038", "MD039"] {
                    assert_eq!(stderr.matches(rule).count(), 2, "{stderr}");
                }
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}
