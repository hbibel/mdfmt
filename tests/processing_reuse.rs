use mdfmt::{
    FormatOptions, Source,
    config::{FrontMatterPattern, ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(rules: Value, pattern: Option<&str>) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        front_matter: pattern.map(|pattern| FrontMatterPattern::new(pattern).unwrap()),
        ..Default::default()
    }
}

#[test]
fn context_reuse_keeps_equal_length_changes_independent() {
    let enabled = config(json!({"default":false,"MD026":true}), None);
    let disabled = config(json!({"default":false,"MD026":false}), None);
    let disabled_inline = "<!-- markdownlint-disable MD026 -->\n# Item!\n";
    let enabled_inline = "<!-- markdownlint-enable  MD026 -->\n# Item!\n";
    assert_eq!(disabled_inline.len(), enabled_inline.len());
    for _ in 0..3 {
        for (input, expected, settings) in [
            ("# Item!\n", "# Item\n", &enabled),
            ("- Item!\n", "- Item!\n", &enabled),
            ("# Item!\n", "# Item!\n", &disabled),
            (disabled_inline, disabled_inline, &enabled),
            (
                enabled_inline,
                "<!-- markdownlint-enable  MD026 -->\n# Item\n",
                &enabled,
            ),
        ] {
            let result = format_with_config(input, &FormatOptions::default(), settings).unwrap();
            assert_eq!(result.output, expected);
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            let again =
                format_with_config(&result.output, &FormatOptions::default(), settings).unwrap();
            assert_eq!(again.output, result.output);
            assert!(again.edits.is_empty());
        }
    }
    let input = "%%%\n# Secret!\n%%%\n\n# Body!\n";
    for (pattern, expected) in [
        (r"^%%%\n[\s\S]*?\n%%%\n", "%%%\n# Secret!\n%%%\n\n# Body\n"),
        (r"^absent", "%%%\n# Secret\n%%%\n\n# Body\n"),
    ] {
        let settings = config(json!({"default":false,"MD026":true}), Some(pattern));
        assert_eq!(
            format_with_config(input, &FormatOptions::default(), &settings)
                .unwrap()
                .output,
            expected
        );
    }
}

#[test]
fn diagnostic_reuse_keeps_metadata_views_independent() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Source>();
    let input = "\u{feff}---\r\n# Secret!\r\nkey:\t😀\r\n---\r\n\r\n<span>\r\n\r\n# Visible!\r\n";
    let shared = std::sync::Arc::new(Source::new(input.into()));
    let threads: Vec<_> = (0..4)
        .map(|index| {
            let shared = shared.clone();
            std::thread::spawn(move || {
                for round in 0..3 {
                    let matched = (round + index) % 2 == 0;
                    let settings = config(
                        json!({"default":false,"MD026":true,"MD024":true}),
                        if matched { None } else { Some("^absent") },
                    );
                    let result =
                        format_with_config(shared.text(), &FormatOptions { fix: false }, &settings)
                            .unwrap();
                    let mut positions: Vec<_> = result
                        .diagnostics
                        .iter()
                        .map(|diagnostic| {
                            let point = shared
                                .position(diagnostic.span.as_ref().unwrap().start)
                                .unwrap();
                            (diagnostic.code.as_str(), point.line, point.column_utf16)
                        })
                        .collect();
                    positions.sort();
                    assert_eq!(
                        positions,
                        if matched {
                            vec![("MD026", 8, 10)]
                        } else {
                            vec![("MD026", 2, 9), ("MD026", 8, 10)]
                        }
                    );
                    assert_eq!(result.output, shared.text());
                    assert!(result.edits.is_empty());
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn repeated_diagnostics_keep_original_positions() {
    let input = "# Same\n\n# Same\n\nalpha * beta * omega\n\n[missing][id]\n";
    let settings = config(
        json!({"default":false,"MD024":true,"MD037":true,"MD052":true}),
        None,
    );
    for _ in 0..4 {
        let result = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
        let source = Source::new(input.into());
        let mut positions: Vec<_> = result
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.clone().unwrap();
                (
                    diagnostic.code.as_str(),
                    source.position(span.start).unwrap().line,
                    span,
                )
            })
            .collect();
        positions.sort_by_key(|(code, line, _)| (*code, *line));
        assert_eq!(
            positions,
            vec![
                ("MD024", 3, 8..8),
                ("MD037", 5, 23..24),
                ("MD037", 5, 28..29),
                ("MD052", 7, 38..51)
            ]
        );
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
    }
}
