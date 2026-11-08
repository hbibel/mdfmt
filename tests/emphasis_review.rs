use mdfmt::{
    FormatOptions, FormatResult, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(rules: Value) -> ResolvedConfig {
    let mut all = json!({"default":false});
    for (key, value) in rules.as_object().unwrap() {
        all.as_object_mut()
            .unwrap()
            .insert(key.clone(), value.clone());
    }
    ResolvedConfig {
        rules: RuleConfig::from_value(all).unwrap(),
        ..Default::default()
    }
}
fn stable(input: &str, expected: &str, rules: Value) -> FormatResult {
    let settings = config(rules);
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let mut applied = input.to_owned();
    for edit in first.edits.iter().rev() {
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(applied, first.output);
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert_eq!(second.diagnostics, first.diagnostics);
    assert!(second.edits.is_empty());
    first
}

#[test]
fn ordinary_nested_attention_must_preserve_enclosing_and_adjacent_runs() {
    for (input, rules) in [
        ("a***one***\n", json!({"MD050":{"style":"underscore"}})),
        ("a***one* two**\n", json!({"MD049":{"style":"underscore"}})),
        ("a*__one__*\n", json!({"MD050":{"style":"asterisk"}})),
        ("a**_one_ two**\n", json!({"MD049":{"style":"asterisk"}})),
    ] {
        let result = stable(input, input, rules);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 2, "{input:?}");
    }
}

#[test]
fn recovered_attention_tokens_do_not_require_native_pairing() {
    for (input, rules, spans) in [
        (
            "__one__****two**\n",
            json!({"MD050":{"style":"underscore"}}),
            vec![9..11, 14..16],
        ),
        (
            "_a 😀_b_\n",
            json!({"MD049":{"style":"asterisk"}}),
            vec![0..1, 9..10],
        ),
        (
            ">**!x!*\n",
            json!({"MD049":{"style":"underscore"}}),
            vec![2..3, 6..7],
        ),
        (
            "|**!x!*|\n|---|\n",
            json!({"MD049":{"style":"underscore"}}),
            vec![2..3, 6..7],
        ),
    ] {
        let settings = config(rules);
        let raw = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
        assert_eq!(raw.output, input);
        assert_eq!(
            raw.diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            spans
        );
    }
    let input = "<!-- markdownlint-disable-next-line MD049 -->\n_a 😀_b_\n\n*two*\n";
    let first = stable(
        input,
        "<!-- markdownlint-disable-next-line MD049 -->\n_a 😀_b_\n\n_two_\n",
        json!({"MD049":true}),
    );
    assert!(first.diagnostics.is_empty());
}

#[test]
fn image_label_recovery_uses_document_reference_definitions() {
    let input = "![[a][*id*]](img)\n\n[*id*]: /u\n\n_one_\n";
    for options in [json!(true), json!({"style":"underscore"})] {
        let first = stable(input, input, json!({"MD049":options}));
        assert!(first.diagnostics.is_empty());
        assert!(first.edits.is_empty());
    }
}

#[test]
fn nested_quoted_images_keep_original_container_context() {
    for (input, expected, rule) in [
        (
            "> ![![a\n> _b_](inner)](outer)\n\n*two*\n",
            "> ![![a\n> _b_](inner)](outer)\n\n_two_\n",
            "MD049",
        ),
        (
            "> ![![a\n> __b__](inner)](outer)\n\n**two**\n",
            "> ![![a\n> __b__](inner)](outer)\n\n__two__\n",
            "MD050",
        ),
    ] {
        let first = stable(input, expected, json!({rule:true}));
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.edits.len(), 2);
        let source = Source::new(input.into());
        assert!(
            first
                .edits
                .iter()
                .all(|e| source.position(e.span.start).unwrap().line == 4)
        );
    }
}

#[test]
fn heading_punctuation_unlocks_markers_without_creating_new_conflicts() {
    for (input, expected, rule, punctuation, remaining) in [
        ("# *one*x\n", "# _one_\n", "MD049", "x", 0),
        ("# **one**x\n", "# __one__\n", "MD050", "x", 0),
        ("# *one*_\n", "# *one*\n", "MD049", "_", 2),
        ("# **one**_\n", "# **one**\n", "MD050", "_", 2),
    ] {
        let first = stable(
            input,
            expected,
            json!({rule:{"style":"underscore"},"MD026":{"punctuation":punctuation}}),
        );
        assert_eq!(first.diagnostics.len(), remaining);
        assert!(first.diagnostics.iter().all(|d| d.code == rule));
        assert!(first.edits.iter().any(|e| e.origin.contains("MD026")));
    }
    for (input, expected, rule) in [
        ("# _one_\n", "# _one\n", "MD049"),
        ("# __one__\n", "# __one\n", "MD050"),
    ] {
        let first = stable(
            input,
            expected,
            json!({rule:{"style":"asterisk"},"MD026":{"punctuation":"_"}}),
        );
        assert!(first.diagnostics.is_empty());
        assert!(first.edits.iter().all(|edit| edit.origin == "MD026"));
    }
}
