use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format, format_with_config,
};

#[test]
fn disabled_fixes_preserve_fixtures_without_claiming_lint_coverage() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/parser/cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let disabled = ResolvedConfig {
            rules: RuleConfig::from_value(serde_json::json!({"default": false})).unwrap(),
            ..Default::default()
        };
        for (fix, config) in [(false, ResolvedConfig::default()), (true, disabled)] {
            let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
            assert_eq!(result.output, input, "{}", case["id"]);
            assert!(!result.changed(input));
            assert!(result.edits.is_empty());
            assert_eq!(
                result.coverage.implemented_rules,
                vec![
                    "MD004", "MD009", "MD010", "MD012", "MD013", "MD019", "MD021", "MD023",
                    "MD026", "MD035", "MD040", "MD042", "MD045", "MD047"
                ]
            );
            assert!(!result.coverage.complete);
        }
    }
}

#[test]
fn disabled_fixes_preserve_empty_and_non_newline_terminated_inputs() {
    for text in [
        "",
        "\u{feff}",
        "no final newline",
        "\r\n",
        "\u{2028}line\u{2029}",
    ] {
        assert_eq!(
            format(text, &FormatOptions { fix: false }).unwrap().output,
            text
        );
    }
}
