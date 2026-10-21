use mdfmt::{FormatOptions, format};

#[test]
fn production_preserves_fixtures_without_claiming_lint_coverage() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/parser/cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        for fix in [true, false] {
            let result = format(input, &FormatOptions { fix }).unwrap();
            assert_eq!(result.output, input, "{}", case["id"]);
            assert!(!result.changed(input));
            assert!(result.edits.is_empty());
            assert!(result.diagnostics.is_empty());
            assert!(result.coverage.implemented_rules.is_empty());
            assert!(!result.coverage.complete);
        }
    }
}

#[test]
fn preserves_empty_and_non_newline_terminated_inputs() {
    for text in [
        "",
        "\u{feff}",
        "no final newline",
        "\r\n",
        "\u{2028}line\u{2029}",
    ] {
        assert_eq!(
            format(text, &FormatOptions::default()).unwrap().output,
            text
        );
    }
}
