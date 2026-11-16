use mdfmt::{
    FormatOptions, ProcessingError,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
use std::{fs, process::Command};

fn settings(value: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(value).unwrap(),
        ..Default::default()
    }
}
fn findings(input: &str, config: &ResolvedConfig) -> Vec<(String, usize)> {
    let result = format_with_config(input, &FormatOptions { fix: false }, config).unwrap();
    assert_eq!(result.output, input);
    let source = mdfmt::Source::new(input.into());
    result
        .diagnostics
        .into_iter()
        .map(|finding| {
            (
                finding.code,
                source.position(finding.span.unwrap().start).unwrap().line,
            )
        })
        .collect()
}

#[test]
fn configured_patterns_control_real_rule_actions() {
    for (rule, body, expected) in [
        ("MD001", "### Deep", vec![("MD001".to_string(), 5)]),
        ("MD025", "# Heading", vec![("MD025".to_string(), 5)]),
        ("MD041", "Ordinary prose.", vec![]),
    ] {
        let input = format!("---\nname: Demo\n---\n\n{body}\n");
        assert_eq!(
            findings(
                &input,
                &settings(json!({"default":false,rule:{"front_matter_title":"^NAME:"}}))
            ),
            expected
        );
    }
    let input = "# Title\n\n[ignored](#SKIP) [case](#TITLE) [bad](#missing)\n";
    let config =
        settings(json!({"default":false,"link-fragments":{"ignored_pattern":"^(SKIP|TITLE)$"}}));
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD051");
    assert!(result.diagnostics[0].message.contains("missing"));
    let config = settings(json!({"default":false,"MD051":{"ignored_pattern":"^SKIP$"}}));
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(
        result.output,
        "# Title\n\n[ignored](#SKIP) [case](#title) [bad](#missing)\n"
    );
    let again = format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(again.output, result.output);
    assert!(again.edits.is_empty());
    let input = "<!-- markdownlint-configure-file {\"required-headings\":{\"headings\":[\"% ^Intro$\"]}} -->\n# Intro\n";
    assert!(findings(input, &settings(json!({"default":false}))).is_empty());
}

#[test]
fn rule_patterns_match_pinned_flags_and_utf16() {
    for (text, pattern, match_case, expected) in [
        ("K", "^k$", false, 0),
        ("K", "^k$", true, 1),
        ("😀", "^.$", true, 0),
        ("😀", "^\\uD83D$", true, 1),
        ("prefix tail", "(?<=prefix )tail", false, 0),
        ("repeat repeat", "^(\\w+) \\1$", false, 0),
    ] {
        let config = settings(
            json!({"default":false,"MD043":{"headings":[format!("% {pattern}")],"match_case":match_case}}),
        );
        assert_eq!(
            findings(&format!("# {text}\n"), &config).len(),
            expected,
            "{pattern}"
        );
    }
    for (pattern, expected) in [("^.$", 1), ("^..$", 0), ("^\\uD83D", 0)] {
        let config = settings(json!({"default":false,"MD051":{"ignored_pattern":pattern}}));
        assert_eq!(
            findings("[link](#😀)\n", &config).len(),
            expected,
            "{pattern}"
        );
    }
    assert_eq!(
        findings(
            "---\nK: value\n---\n\nOrdinary prose.\n",
            &settings(json!({"default":false,"MD041":{"front_matter_title":"^k:"}}))
        )
        .len(),
        1
    );
    let rows: Value =
        serde_json::from_str(include_str!("fixtures/config-patterns/cases.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let config = settings(row["config"].clone());
        let mut actual: Vec<_> = findings(row["input"].as_str().unwrap(), &config)
            .into_iter()
            .map(|(rule, line)| json!({"rule":rule,"line":line}))
            .collect();
        actual.sort_by_key(|value| {
            (
                value["line"].as_u64().unwrap(),
                value["rule"].as_str().unwrap().to_owned(),
            )
        });
        assert_eq!(
            actual,
            row["findings"].as_array().unwrap().clone(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn shorthand_folding_respects_consumer_unicode_flags() {
    for key in ["K", "ſ"] {
        for (pattern, title) in [
            (r"^\w+:", false),
            (r"^[\w]+:", false),
            (r"^\W+:", true),
            (r"^[\W]+:", true),
        ] {
            for (rule, body, count) in [
                ("MD041", "Ordinary prose.", usize::from(!title)),
                ("MD001", "### Deep", usize::from(title)),
                ("MD025", "# Heading", usize::from(title)),
            ] {
                assert_eq!(
                    findings(
                        &format!("---\n{key}: Title\n---\n\n{body}\n"),
                        &settings(json!({"default":false,rule:{"front_matter_title":pattern}}))
                    )
                    .len(),
                    count,
                    "{key} {rule} {pattern}"
                );
            }
        }
        for (pattern, count) in [(r"^\w$", 0), (r"^[\w]$", 0), (r"^\W$", 1), (r"^[\W]$", 1)] {
            assert_eq!(
                findings(
                    &format!("# {key}\n"),
                    &settings(
                        json!({"default":false,"MD043":{"headings":[format!("% {pattern}")]}})
                    )
                )
                .len(),
                count,
                "iu {key} {pattern}"
            );
        }
    }
}

#[test]
fn pattern_errors_are_transactional_and_recoverable() {
    for value in [
        json!({"MD041":{"front_matter_title":"["}}),
        json!({"MD051":{"ignored_pattern":"["}}),
        json!({"MD043":{"headings":["% ("]}}),
        json!({"MD051":{"ignored_pattern":"a".repeat(65537)}}),
        json!({"MD051":{"ignored_pattern":format!("{}a{}","(".repeat(65),")".repeat(65))}}),
    ] {
        let error = RuleConfig::from_value(value).unwrap_err();
        assert_eq!(error.code, "invalid-config");
        assert!(
            error.property.contains("front_matter_title")
                || error.property.contains("ignored_pattern")
                || error.property.contains("headings")
        );
    }
    let input = format!("[link](#{})\n", "a".repeat(200));
    let config = settings(json!({"default":false,"MD051":{"ignored_pattern":"(a+)+b"}}));
    let start = std::time::Instant::now();
    let error = format_with_config(&input, &FormatOptions::default(), &config).unwrap_err();
    assert!(matches!(error, ProcessingError::WorkLimit(_)), "{error}");
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    let disabled =
        settings(json!({"default":false,"MD051":{"enabled":false,"ignored_pattern":"(a+)+b"}}));
    assert!(findings(&input, &disabled).is_empty());
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".markdownlint.json"),
        json!({"default":false,"MD051":{"ignored_pattern":"(a+)+b"}}).to_string(),
    )
    .unwrap();
    fs::write(dir.path().join("doc.md"), &input).unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("work limit"));
        assert_eq!(
            fs::read_to_string(dir.path().join("doc.md")).unwrap(),
            input
        );
    }
    let safe = settings(json!({"default":false,"MD051":{"ignored_pattern":"^safe$"}}));
    assert!(findings("[link](#safe)\n", &safe).is_empty());
    assert!(RuleConfig::from_value(json!({"MD036":{"punctuation":"z-a"}})).is_err());
}

#[test]
fn document_pattern_cache_is_bounded() {
    let headings: Vec<_> = (0..257)
        .map(|index| format!("% ^Heading {index}$"))
        .collect();
    let input = (0..257)
        .map(|index| format!("# Heading {index}\n\n"))
        .collect::<String>();
    let config = settings(json!({"default":false,"MD043":{"headings":headings}}));
    let error = format_with_config(&input, &FormatOptions { fix: false }, &config).unwrap_err();
    assert!(matches!(error, ProcessingError::WorkLimit(_)));
    assert!(error.to_string().contains("cache"));
}
