use mdfmt::{
    FormatOptions, FormatResult, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn stable(input: &str, rules: Value) -> FormatResult {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    for fix in [false, true] {
        let again = format_with_config(&first.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(again.output, input);
        assert_eq!(again.diagnostics, first.diagnostics);
        assert!(again.edits.is_empty());
    }
    first
}

#[test]
fn nested_resource_labels_contribute_data_without_destinations_or_false_resolution() {
    for (input, end) in [
        ("[text][a![foo](url)b]\n\n[ab]: /dest\n", 21),
        ("[text][a[foo](url)b]\n\n[ab]: /dest\n", 19),
    ] {
        let result = stable(input, json!({"default":false,"MD052":true,"MD053":true}));
        assert_eq!(result.diagnostics.len(), 2, "{result:?}");
        let missing = result
            .diagnostics
            .iter()
            .find(|d| d.code == "MD052")
            .unwrap();
        assert_eq!(missing.span, Some(0..end));
        assert!(
            missing
                .message
                .contains("Missing link or image reference definition: \"afoob\"")
        );
        let unused = result
            .diagnostics
            .iter()
            .find(|d| d.code == "MD053")
            .unwrap();
        assert!(
            unused
                .message
                .contains("Unused link or image reference definition: \"ab\"")
        );
        assert_eq!(&input[unused.span.clone().unwrap()], "[ab]: /dest");
    }
    let result = stable(
        "[[x](u)](v)\n",
        json!({"default":false,"MD052":{"shortcut_syntax":true,"ignored_labels":[]}}),
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(0..7));
    assert!(result.diagnostics[0].message.contains("definition: \"x\""));
}

#[test]
fn math_brackets_do_not_replace_owned_label_boundaries_or_reference_uses() {
    for label in ["$[$", "$]$", "vector $[x$", "vector $x]$"] {
        for input in [
            format!("[{label}](url)\n"),
            format!("![{label}](img)\n"),
            format!("[{label}][id]\n\n[id]: /u\n"),
            format!("![{label}][id]\n\n[id]: /u\n"),
        ] {
            let result = stable(
                &input,
                json!({"default":false,"MD052":true,"MD053":true,"MD054":{"inline":false,"full":false}}),
            );
            assert_eq!(result.diagnostics.len(), 1, "{input:?}: {result:?}");
            assert_eq!(result.diagnostics[0].code, "MD054");
        }
    }
}

#[test]
fn quoted_image_recovery_preserves_helper_keys_and_resolved_native_style() {
    let input = "> ![[text][a\n> b]](img)\n>\n> [a\n> b]: /url\n";
    for rules in [
        json!({"default":false,"MD052":true}),
        json!({"default":false,"MD053":true}),
        json!({"default":false,"MD054":{"full":false}}),
        json!({"default":false,"MD052":true,"MD053":true,"MD054":{"full":false}}),
    ] {
        let enabled = rules.as_object().unwrap().len() - 1;
        let result = stable(input, rules);
        assert_eq!(result.diagnostics.len(), enabled, "{result:?}");
        let source = Source::new(input.into());
        for diagnostic in &result.diagnostics {
            let span = diagnostic.span.clone().unwrap();
            match diagnostic.code.as_str() {
                "MD052" => {
                    assert_eq!(span, 4..12);
                    assert!(diagnostic.message.contains("definition: \"a b\""));
                }
                "MD053" => {
                    assert_eq!(source.position(span.start).unwrap().line, 4);
                    assert_eq!(&input[span], "> [a");
                    assert!(diagnostic.message.contains("definition: \"a > b\""));
                }
                "MD054" => assert_eq!(span, 0..0),
                _ => panic!("unexpected diagnostic: {diagnostic:?}"),
            }
        }
    }
}
