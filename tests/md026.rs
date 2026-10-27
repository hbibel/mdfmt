use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format, format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD026":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let config = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output, "{input:?}");
    assert!(second.edits.is_empty(), "{input:?}");
    for edit in &first.edits {
        assert_eq!(edit.origin, "MD026");
        assert!(edit.replacement.is_empty());
    }
}

#[test]
fn default_and_configured_punctuation_are_fixed() {
    assert_eq!(
        format("# Hello!", &FormatOptions::default())
            .unwrap()
            .output,
        "# Hello"
    );
    for (input, expected) in [
        ("# Hello!", "# Hello"),
        ("## Hello.,;:!。，；：！", "## Hello"),
        ("Hello!\n---", "Hello\n---"),
        ("# Hello!  ###  ", "# Hello  ###  "),
        ("> # Hello! ###", "> # Hello ###"),
        ("- # Hello!\n", "- # Hello\n"),
        ("# ! ###", "#  ###"),
        ("# !!!", "# "),
        ("# Hello?", "# Hello?"),
        ("# Hello! !", "# Hello"),
        ("# Hello &amp;!", "# Hello &amp"),
        ("# Hello :smile:!", "# Hello :smile"),
    ] {
        fixed(input, expected, json!(true));
    }
    fixed("# Hello?!", "# Hello?!", json!({"punctuation":"?"}));
    fixed("# Hello?", "# Hello", json!({"punctuation":"?"}));
    fixed("# Hello!", "# Hello!", json!({"punctuation":""}));
    fixed("# Hello!", "# Hello!", json!({"punctuation":".,;:"}));
    fixed("# Hello!", "# Hello!", json!(false));
}

#[test]
fn exceptions_and_nonheadings_are_preserved() {
    for input in [
        "# Hello &amp;",
        "# Hello &#123;",
        "# Hello &#xabc;",
        "# Hello &unknown;",
        "# Hello :smile:",
        "# Hello :+1:",
        "# Hello :clock1234:",
        "Hello! \n---",
        "Hello!\t\n---",
        "# Hello!\u{a0}",
        "# *Hello!*",
        "# `Hello!`",
        "# [Hello!](url)",
        "# Hello! <!-- comment -->",
        "Hello!",
        "```md\n# Hello!\n```",
        "    # Hello!",
        "| # Hello! |\n| -------- |\n| data |",
        "---\ntitle: Hello!\n---\n",
    ] {
        fixed(input, input, json!(true));
    }
    fixed("# Hello &x;", "# Hello &x", json!(true));
    fixed("# Hello :invalid9:", "# Hello :invalid9", json!(true));
}

#[test]
fn source_boundaries_preserve_containers_bom_and_newlines() {
    for (input, expected) in [
        ("\u{feff}# 😀Hello!\r\n", "\u{feff}# 😀Hello\r\n"),
        ("First\nsecond!\n---\n", "First\nsecond\n---\n"),
        (
            "> First\r\n> second!\r\n> ---\r\n",
            "> First\r\n> second\r\n> ---\r\n",
        ),
        ("- First\n  second!\n  ---\n", "- First\n  second\n  ---\n"),
        (
            "---\ntitle: Hi!\n---\n\n# Hello!\n",
            "---\ntitle: Hi!\n---\n\n# Hello\n",
        ),
        ("# Hello\u{a0}!", "# Hello"),
        ("# Hello\\!", "# Hello\\"),
    ] {
        fixed(input, expected, json!(true));
    }
}

#[test]
fn inline_permissions_and_aliases_gate_edits_and_findings() {
    for input in [
        "<!-- markdownlint-disable-file MD026 -->\n# Hello!",
        "<!-- markdownlint-disable-next-line no-trailing-punctuation -->\n# Hello!",
        "<!-- markdownlint-disable headings -->\n# Hello!",
    ] {
        fixed(input, input, json!(true));
    }
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"no-trailing-punctuation":true}))
            .unwrap(),
        ..Default::default()
    };
    assert_eq!(
        format_with_config("# Hello!", &FormatOptions::default(), &config)
            .unwrap()
            .output,
        "# Hello"
    );
}

#[test]
fn disabled_fixes_report_configured_severity_and_output_spans() {
    let input = "# 😀Hello !\r\n";
    let config = config(json!({"severity":"warning"}));
    let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, "MD026");
    assert_eq!(diagnostic.rule_id.as_deref(), Some("MD026"));
    assert_eq!(diagnostic.severity, Severity::Warning);
    let span = diagnostic.span.clone().unwrap();
    assert_eq!(&input[span.clone()], " !");
    let position = Source::new(input.into()).position(span.start).unwrap();
    assert_eq!((position.line, position.column_utf16), (1, 10));
    let mut config = config;
    config.fix = false;
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &config)
            .unwrap()
            .diagnostics
            .len(),
        1
    );
}

#[test]
fn heading_fixes_and_multiline_reflow_share_a_safe_batch() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD026":true,"MD013":{"line_length":10}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "# Title!\n\n- alpha\n  beta gamma\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "# Title\n\n- alpha\n  beta\n  gamma\n");
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        result.output
    );
}
