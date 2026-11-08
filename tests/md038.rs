use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD038":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn code_span_findings_preserve_literal_code_value() {
    let input = "`  text  `\n";
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
            [1..3, 7..9]
        );
        assert!(result.diagnostics.iter().all(|d| d.code == "MD038"
            && d.message == "Spaces inside code span elements [Context: \"`  text  `\"]"));
        let again = format_with_config(&result.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(again.diagnostics, result.diagnostics);
        assert_eq!(again.output, input);
        assert!(again.edits.is_empty());
    }
    for input in ["` text `\n", "`   `\n", "`` `x` ``\n"] {
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn quoted_code_spans_keep_original_physical_positions() {
    let input = "> `  first\n> last  `\n";
    let source = Source::new(input.into());
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    let positions: Vec<_> = result
        .diagnostics
        .iter()
        .map(|d| {
            let span = d.span.clone().unwrap();
            let start = source.position(span.start).unwrap();
            assert_eq!(
                d.message,
                "Spaces inside code span elements [Context: \"`  first > last  `\"]"
            );
            (start.line, start.column_utf16, span.len())
        })
        .collect();
    assert_eq!(positions, [(1, 4, 2), (2, 7, 2)]);
}

#[test]
fn newline_padding_never_manufactures_invalid_code_ranges() {
    for (input, spans) in [
        ("`\n a \n`\n", vec![2..3, 4..5]),
        ("`\n  a  \n`\n", vec![2..4, 5..7]),
        ("> `\n>  a \n> `\n", vec![6..7, 8..9]),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            spans
        );
    }
}

#[test]
fn code_space_ranges_contexts_and_permissions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md038-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD038");let span=d.span.clone().unwrap();let p=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(p.line,end.line);json!({"line":p.line,"range":[p.column_utf16-usize::from(p.line==1&&input.starts_with('\u{feff}')),end.column_utf16-p.column_utf16],"context":d.message.strip_prefix("Spaces inside code span elements [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"detail":Value::Null,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
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
fn code_bytes_remain_indivisible_during_reflow_and_marker_changes() {
    let input = "_one_ alpha beta `  text  ` gamma delta\n";
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD038":true,"MD013":{"line_length":20},"MD049":{"style":"asterisk"},"MD050":{"style":"asterisk"}})).unwrap(),..Default::default()};
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, "*one* alpha beta\n`  text  ` gamma\ndelta\n");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "MD038")
            .count(),
        2
    );
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}
