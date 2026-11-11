use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

#[test]
fn soft_break_before_indivisible_links_consumes_existing_container_prefixes() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD013":{"line_length":30}})).unwrap(),
        ..Default::default()
    };
    for input in [
        "- a paragraph with enough\n  words to need wrapping\n  [link](url)\n",
        "> a paragraph with enough\n> words to need wrapping\n> [link](url)\n",
        "- a paragraph with enough\n  words to need wrapping\n  `literal code`\n",
    ] {
        for input in [
            input.to_owned(),
            format!("\u{feff}{}", input.replace('\n', "\r\n")),
        ] {
            for _ in 0..2 {
                let result =
                    format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
                assert_eq!(result.output, input);
                assert!(result.edits.is_empty());
                assert!(result.diagnostics.is_empty());
            }
        }
    }
}
