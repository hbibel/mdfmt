use super::Pattern;
use crate::config::brace;
use serde_json::{Value, json};

#[test]
fn compiler_matches_all_recorded_filtering_observations() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/configuration/glob-cases.json"
    ))
    .unwrap();
    let observations: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/configuration/glob-observations.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    for observation in observations.as_array().unwrap() {
        let pattern = observation["pattern"].as_str().unwrap();
        let compiled = Pattern::new(pattern, false).unwrap();
        let actual: Vec<_> = cases["paths"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|path| compiled.filter_matches(path.as_str().unwrap()).unwrap())
            .collect();
        if json!(actual) != observation["filtering"] {
            differences.push(format!(
                "{pattern:?}: {} != {}",
                json!(actual),
                observation["filtering"]
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn native_regex_work_exhaustion_is_an_error_and_recoverable() {
    let hostile = Pattern::new("+(a*)b", false).unwrap();
    let error = hostile
        .regex_matches(&format!("{}!", "a".repeat(40)))
        .unwrap_err();
    assert_eq!(error.code, "glob-work-limit");
    assert!(
        Pattern::new("*.md", false)
            .unwrap()
            .filter_matches("a.md")
            .unwrap()
    );
}

#[test]
fn compiler_matches_all_recorded_enumeration_observations() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/configuration/glob-cases.json"
    ))
    .unwrap();
    let observations: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/configuration/glob-observations.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    for observation in observations.as_array().unwrap() {
        let pattern = observation["pattern"].as_str().unwrap();
        let expanded = brace::enumerate(pattern).unwrap();
        assert_eq!(json!(expanded), observation["expanded"], "{pattern}");
        let compiled: Vec<_> = expanded
            .iter()
            .map(|p| Pattern::new(p, true).unwrap())
            .collect();
        let actual: Vec<_> = cases["paths"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|path| {
                compiled
                    .iter()
                    .any(|p| p.regex_matches(path.as_str().unwrap()).unwrap())
            })
            .collect();
        if json!(actual) != observation["enumeration"] {
            differences.push(format!(
                "{pattern:?}: {} != {}",
                json!(actual),
                observation["enumeration"]
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn compiler_matches_generated_upstream_observations() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/configuration/generated-globs.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let pattern = case["pattern"].as_str().unwrap();
        let expanded = brace::enumerate(pattern).unwrap();
        if json!(expanded) != case["expanded"] {
            differences.push(format!(
                "expansion {pattern:?}: {} != {}",
                json!(expanded),
                case["expanded"]
            ));
            continue;
        }
        for context in ["enumeration", "filtering"] {
            let patterns = if context == "enumeration" {
                expanded.clone()
            } else {
                vec![pattern.into()]
            };
            let compiled: Vec<_> = patterns
                .iter()
                .filter(|p| !p.is_empty())
                .map(|p| Pattern::new(p, context == "enumeration").unwrap())
                .collect();
            let actual: Vec<_> = fixture["paths"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|path| {
                    compiled.iter().any(|p| {
                        if context == "enumeration" {
                            p.regex_matches(path.as_str().unwrap()).unwrap()
                        } else {
                            p.filter_matches(path.as_str().unwrap()).unwrap()
                        }
                    })
                })
                .collect();
            if json!(actual) != case[context] {
                differences.push(format!(
                    "{context} {pattern:?}: {} != {}",
                    json!(actual),
                    case[context]
                ));
            }
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn compiler_and_enumerator_reject_excessive_work_before_allocating_output() {
    assert_eq!(
        Pattern::new(&"a".repeat(65_537), false).err().unwrap().code,
        "invalid-config"
    );
    let nested = format!("{}a{}", "@(".repeat(65), ")".repeat(65));
    assert_eq!(
        Pattern::new(&nested, false).err().unwrap().code,
        "invalid-config"
    );
    let padded = format!("file{{{:020000}..{:020000}}}.md", 1, 1000);
    assert_eq!(
        brace::enumerate(&padded).unwrap_err().code,
        "invalid-config"
    );
}

#[test]
fn nested_repetition_preserves_successful_backtracking() {
    for pattern in ["+(+(a(a)+)a)a.md", "((aa+)+a)+a.md"] {
        assert!(
            Pattern::new(pattern, false)
                .unwrap()
                .filter_matches("aaaa.md")
                .unwrap(),
            "{pattern}"
        );
    }
}

#[test]
fn escaped_surrogates_remain_utf16_code_units() {
    for pattern in [r"\uD83E\uDD80.md", r"[\uD83E\uDD80][\uD83E\uDD80].md"] {
        assert!(
            Pattern::new(pattern, false)
                .unwrap()
                .filter_matches("🦀.md")
                .unwrap(),
            "{pattern}"
        );
    }
    assert!(
        Pattern::new("{🦀..🦞}.md", false)
            .unwrap()
            .filter_matches("🦀..🦞.md")
            .unwrap()
    );
}

#[test]
fn quotes_inside_plain_tokens_remain_literal() {
    let compiled = Pattern::new("@(a\"b\").md", false).unwrap();
    assert!(compiled.filter_matches("a\"b\".md").unwrap());
    assert!(!compiled.filter_matches("ab.md").unwrap());
}
