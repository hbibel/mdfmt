use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD031":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn fence_separators_preserve_code_and_settle_once() {
    let input = "before\n```rust\nx\n```\nafter\n";
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
        [2, 4]
    );
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, "before\n\n```rust\nx\n```\n\nafter\n");
    assert!(first.diagnostics.is_empty());
    assert!(
        first
            .edits
            .iter()
            .all(|e| e.span.is_empty() && e.origin == "MD031")
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert!(again.edits.is_empty());
}

#[test]
fn unterminated_quote_fence_uses_token_end_not_last_code_content_line() {
    for (input, expected) in [
        ("> ```\n- item\n>\n", vec![]),
        ("> ```\n> x\n- item\n# H\n", vec![3]),
    ] {
        for fix in [false, true] {
            let result =
                format_with_config(input, &FormatOptions { fix }, &config(json!(true))).unwrap();
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
                expected
            );
            if !expected.is_empty() {
                assert_eq!(
                    result.diagnostics[0].message,
                    "Fenced code blocks should be surrounded by blank lines [Context: \"- item\"]"
                );
            }
        }
    }
}

#[test]
fn fence_boundaries_and_list_exceptions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md031-observations.json"
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
        );
        if row["nativeError"] == "parse-error" {
            assert!(
                matches!(result,Err(mdfmt::ProcessingError::Parse(message)) if message.contains("native parser panicked"))
            );
            continue;
        }
        let result = result.unwrap();
        assert_eq!(result.output, input);
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{
            assert_eq!(d.code,"MD031");let span=d.span.clone().unwrap();assert!(span.is_empty());
            let position=source.position(span.start).unwrap();assert_eq!(position.column_utf16,1+usize::from(position.line==1&&input.starts_with('\u{feff}')));
            json!({"line":position.line,"message":d.message,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        let expected:Vec<_>=row["findings"].as_array().unwrap().iter().map(|f|json!({"line":f["line"],"message":format!("Fenced code blocks should be surrounded by blank lines [Context: \"{}\"]",f["context"].as_str().unwrap()),"severity":f["severity"]})).collect();
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
fn list_tightness_and_unrelated_next_line_targets_are_preserved() {
    let input = "- before\n  ```rust\n  x\n  ```\n  after\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 2);
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"list_items":false})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.is_empty());
    let input = "before <!-- markdownlint-disable-next-line MD040 -->\n```\nx\n```\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD031":true,"MD040":true})).unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD031");
    assert!(result.edits.is_empty());
}

#[test]
fn fence_style_and_prose_reflow_finish_before_separator_positions() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD031":true,"MD048":{"style":"tilde"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(
        "before\n```rust\nx\n```\nafter\n",
        &FormatOptions::default(),
        &settings,
    )
    .unwrap();
    assert_eq!(result.output, "before\n\n~~~rust\nx\n~~~\n\nafter\n");
    assert!(result.diagnostics.is_empty());
    let input =
        "a paragraph with several words to reflow\n\n- item\n  ```rust\n  x\n  ```\n  tail\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD031":{"severity":"warning"},"MD013":{"line_length":20}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(
        first
            .output
            .starts_with("a paragraph with\nseveral words to\nreflow\n\n")
    );
    assert!(
        first
            .output
            .ends_with("- item\n  ```rust\n  x\n  ```\n  tail\n")
    );
    let source = Source::new(first.output.clone());
    assert_eq!(
        first
            .diagnostics
            .iter()
            .map(|d| source
                .position(d.span.as_ref().unwrap().start)
                .unwrap()
                .line)
            .collect::<Vec<_>>(),
        [6, 8]
    );
    assert!(
        first
            .diagnostics
            .iter()
            .all(|d| d.code == "MD031" && d.severity == Severity::Warning)
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert_eq!(again.diagnostics, first.diagnostics);
    assert!(again.edits.is_empty());
}
