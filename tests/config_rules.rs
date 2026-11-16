use mdfmt::{Severity, config::RuleConfig};
use serde_json::json;

#[test]
fn ordered_aliases_tags_and_severities() {
    let rules = RuleConfig::from_value(
        serde_json::from_str(
            r#"{"default":"warning","MD013":false,"line_length":{"line_length":42}}"#,
        )
        .unwrap(),
    )
    .unwrap();
    let states = rules.effective();
    assert!(states["MD013"].enabled);
    assert_eq!(states["MD013"].severity, Severity::Error);
    assert_eq!(states["MD013"].options["line_length"], 42);
    assert_eq!(states["MD026"].severity, Severity::Warning);
    let rules = RuleConfig::from_value(json!({"default": false, "hEaDiNgS": true})).unwrap();
    assert!(rules.effective()["MD026"].enabled);
    assert!(!rules.effective()["MD013"].enabled);
}

#[test]
fn invalid_and_deferred_settings_are_explicit_errors() {
    for value in [
        json!({"MD999":true}),
        json!({"MD013":{"typo":80}}),
        json!({"MD013":{"line_length":"80"}}),
        json!({"MD026":{"enabled":2}}),
        json!({"default":{}}),
    ] {
        assert_eq!(
            RuleConfig::from_value(value).unwrap_err().code,
            "invalid-config"
        );
    }
    for value in [
        json!({"MD041":{"front_matter_title":"["}}),
        json!({"link-fragments":{"ignored_pattern":"["}}),
    ] {
        assert_eq!(
            RuleConfig::from_value(value).unwrap_err().code,
            "invalid-config"
        );
    }
    assert!(RuleConfig::from_value(json!({"MD051":{"ignored_pattern":""}})).is_ok());
}

#[test]
fn shallow_merging_preserves_key_positions_and_source_defaults() {
    let mut parent = RuleConfig::from_value(
        serde_json::from_str(
            r#"{"default":false,"MD013":{"strict":true,"line_length":60},"line_length":false}"#,
        )
        .unwrap(),
    )
    .unwrap();
    parent.merge(&RuleConfig::from_value(json!({"MD013":{"line_length":40}})).unwrap());
    assert!(!parent.effective()["MD013"].enabled);
    let rules = RuleConfig::from_value(json!({"MD013":{"line_length":40}})).unwrap();
    assert!(
        !rules.effective()["MD013"]
            .options
            .contains_key("heading_line_length")
    );
}

#[test]
fn option_constraints_match_the_pinned_schema() {
    for value in [
        json!({"MD013":{"line_length":0}}),
        json!({"MD007":{"indent":-1}}),
        json!({"MD003":{"style":"invented"}}),
        json!({"MD044":{"names":[42]}}),
        json!({"MD022":{"lines_above":["one"]}}),
        json!({"MD043":{"headings":["not a heading"]}}),
    ] {
        assert_eq!(
            RuleConfig::from_value(value.clone()).unwrap_err().code,
            "invalid-config",
            "{value}"
        );
    }
    for value in [
        json!({"MD022":{"lines_above":[0,1]}}),
        json!({"MD043":{"headings":["# Title","*","^", "?"]}}),
    ] {
        assert!(RuleConfig::from_value(value).is_ok());
    }
}
