use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[test]
fn applicable_md013_acceptance_cases_are_real_idempotent_formatter_tests() {
    let corpus: Value =
        serde_json::from_str(include_str!("../docs/compatibility/acceptance.json")).unwrap();
    let mut tested = 0;
    for case in corpus["cases"].as_array().unwrap() {
        if case["config"].get("MD013").is_none()
            || case.get("files").is_some()
            || case["formatterScenario"]
                .as_str()
                .is_some_and(|scenario| scenario != "unavailable")
        {
            continue;
        }
        let config = ResolvedConfig {
            rules: RuleConfig::from_value(case["config"].clone()).unwrap(),
            no_inline_config: case["cliOptions"]["noInlineConfig"]
                .as_bool()
                .unwrap_or(false),
            ..Default::default()
        };
        let input = case["input"].as_str().unwrap();
        let expected = case["expectedOutput"].as_str().unwrap();
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(result.output, expected, "{}", case["id"]);
        let remaining: BTreeSet<_> = result
            .diagnostics
            .iter()
            .filter_map(|d| d.rule_id.as_deref())
            .collect();
        let expected_rules: BTreeSet<_> = case["remainingRuleIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(remaining, expected_rules, "{}", case["id"]);
        let second =
            format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, result.output, "{}", case["id"]);
        assert!(second.edits.is_empty(), "{}", case["id"]);
        tested += 1;
    }
    assert_eq!(tested, 24);
}
