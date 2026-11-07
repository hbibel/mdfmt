use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

#[test]
fn punctuation_classes_reject_regex_injection_and_invalid_ranges() {
    for punctuation in ["]|.*", "z-a", "\\"] {
        assert!(
            RuleConfig::from_value(json!({"MD036":{"punctuation":punctuation}})).is_err(),
            "{punctuation:?}"
        );
    }
}

#[test]
fn content_dependent_findings_match_pinned_ranges_and_contexts() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/content-diagnostics-observations.json"
    ))
    .unwrap();
    let mut mismatches = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let rule = row["rule"].as_str().unwrap();
        let config = ResolvedConfig {
            rules: RuleConfig::from_value(json!({"default":false,rule:row["options"]})).unwrap(),
            ..Default::default()
        };
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        let actual: Vec<_>=result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code,rule);
            let span=finding.span.clone().unwrap();
            assert!(input.get(span.clone()).is_some());
            let position=source.position(span.start).unwrap();
            let range=if span.is_empty(){Value::Null}else{json!([position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),mdfmt::utf16_len(&input[span])])};
            let context=finding.message.split_once(" [Context: \"").map(|(_,tail)|tail.strip_suffix("\"]").unwrap());
            json!({"line":position.line,"range":range,"context":context,"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        if actual != *row["findings"].as_array().unwrap() {
            mismatches.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn findings_follow_final_output_lines_without_rewriting_labels_or_emphasis() {
    let input = "Some words for a long prose paragraph that wraps across several lines.\n\n**Heading**\n\n[here](url)\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD013":{"line_length":20},"MD036":true,"MD059":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert!(first.output.contains("**Heading**\n\n[here](url)"));
    assert_eq!(
        first
            .diagnostics
            .iter()
            .map(|f| f.code.as_str())
            .collect::<Vec<_>>(),
        vec!["MD036", "MD059"]
    );
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, second.output);
    assert_eq!(first.diagnostics, second.diagnostics);
}
