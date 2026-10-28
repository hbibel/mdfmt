use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD010":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let config = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn tab_runs_use_configured_multiplier_not_visual_tab_stops() {
    for (input, expected, options) in [
        ("alpha\tbeta\n", "alpha beta\n", json!(true)),
        (
            "alpha\t\tbeta\n",
            "alpha    beta\n",
            json!({"spaces_per_tab":2}),
        ),
        (
            "alpha\tbeta\n",
            "alpha    beta\n",
            json!({"spaces_per_tab":4}),
        ),
        (
            "alpha \t beta\n",
            "alpha  beta\n",
            json!({"spaces_per_tab":0}),
        ),
        ("\u{feff}😀\tbeta\r\n", "\u{feff}😀 beta\r\n", json!(true)),
    ] {
        fixed(input, expected, options);
    }
}

#[test]
fn meaning_changes_and_protected_source_are_not_forced() {
    for (input, options) in [
        ("alpha\tbeta\n", json!({"spaces_per_tab":0})),
        ("`code\tvalue`\n", json!(true)),
        ("```js\ncode\tvalue\n```\n", json!(true)),
        ("    code\tvalue\n", json!(true)),
        ("| a\tb |\n| --- |\n", json!(true)),
        ("[label\tvalue](url)\n", json!(true)),
        ("-\titem\n", json!(true)),
        ("> 1.\t---\n >  \t# Heading!", json!(true)),
        ("alpha\tbeta\n", json!({"spaces_per_tab":u64::MAX})),
    ] {
        fixed(input, input, options);
    }
}

#[test]
fn diagnostics_match_pinned_runtime_options_and_boundaries() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md010-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        let source = Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD010");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":span.len(),"detail":finding.message.strip_prefix("Hard tabs [").unwrap().strip_suffix(']').unwrap(),
                "severity":if finding.severity == Severity::Warning {"warning"} else {"error"}})
        }).collect();
        assert_eq!(
            actual,
            *row["findings"].as_array().unwrap(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn tab_expansion_and_trailing_whitespace_settle_with_original_edits() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD010":{"spaces_per_tab":2},"MD009":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "alpha\t\t\n";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "alpha\n");
    assert_eq!(first.edits.len(), 1);
    assert_eq!(first.edits[0].span, 5..7);
    assert_eq!(first.edits[0].replacement, "");
    assert_eq!(first.edits[0].origin, "MD009+MD010");
    assert!(first.diagnostics.is_empty());
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn permissions_aliases_and_severity_apply_to_tabs() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-hard-tabs":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "alpha\tbeta\n";
    for fix in [false, true] {
        config.fix = false;
        let result = format_with_config(input, &FormatOptions { fix }, &config).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }
    config.fix = true;
    let input = "<!-- markdownlint-disable-next-line hard_tab -->\nalpha\tbeta\n";
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        input
    );
}

#[test]
fn all_rule_edits_reconstruct_original_source_and_are_stable() {
    let config = ResolvedConfig { rules: RuleConfig::from_value(json!({"default":false,"MD004":{"style":"plus"},"MD009":true,"MD010":true,"MD012":true,"MD013":{"line_length":14},"MD019":true,"MD026":true,"MD035":{"style":"* * *"},"MD047":true})).unwrap(), ..Default::default() };
    let input = "#  Title!\r\n\r\n\r\n- alpha\tbeta gamma delta \r\n\r\n---";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_ne!(first.output, input);
    assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
    let mut applied = String::new();
    let mut cursor = 0;
    for edit in &first.edits {
        applied.push_str(&input[cursor..edit.span.start]);
        applied.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    applied.push_str(&input[cursor..]);
    assert_eq!(applied, first.output);
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn unicode_whitespace_remains_part_of_raw_fence_language() {
    for separator in ['\u{a0}', '\u{2003}'] {
        let input = format!("```js{separator}extra\nalpha\tbeta\n```\n");
        for include_code in [false, true] {
            let config = config(json!({"code_blocks":include_code,"ignore_code_languages":["js"]}));
            let result =
                format_with_config(&input, &FormatOptions { fix: false }, &config).unwrap();
            assert_eq!(result.diagnostics.len(), 1, "{input:?}");
            assert_eq!(
                result.diagnostics[0].span,
                Some(input.find('\t').unwrap()..input.find('\t').unwrap() + 1)
            );
        }
    }
}
