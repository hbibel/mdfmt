use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

fn config(width: usize) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"MD013": {"line_length": width}})).unwrap(),
        ..Default::default()
    }
}

fn check(input: &str, expected: &str, width: usize) {
    let config = config(width);
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, expected, "input: {input:?}");
    assert!(!result.coverage.complete);
    assert_eq!(result.coverage.implemented_rules, vec!["MD013", "MD026"]);
    let second = format_with_config(&result.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, result.output);
    assert!(second.edits.is_empty());
    for edit in result.edits {
        assert_eq!(edit.origin, "MD013");
        assert!(
            input[edit.span]
                .chars()
                .all(|ch| matches!(ch, ' ' | '\r' | '\n' | '>'))
        );
    }
}

#[test]
fn wraps_to_width_even_when_upstream_exempts_the_last_token() {
    check("alpha beta gamma delta\n", "alpha beta\ngamma\ndelta\n", 10);
    check("one two three", "one two\nthree", 9);
    check("alpha beta", "alpha beta", 10);
    check("alpha verylongtoken tail", "alpha\nverylongtoken\ntail", 8);
    check("one   two three", "one   two\nthree", 9);
}

#[test]
fn native_outputs_match_the_pinned_oracle_corpus() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/reflow.json")).unwrap();
    for case in cases.as_array().unwrap() {
        check(
            case["input"].as_str().unwrap(),
            case["output"].as_str().unwrap(),
            case["width"].as_u64().unwrap() as usize,
        );
    }
}

#[test]
fn counts_utf16_units_not_bytes_or_unicode_scalars() {
    check("😀 ab cd", "😀 ab\ncd", 5);
    check("éé ab cd", "éé ab\ncd", 5);
    check("😀 ab", "😀 ab", 5);
}

#[test]
fn inline_source_units_stay_indivisible() {
    check("alpha [a b](url) tail", "alpha\n[a b](url)\ntail", 12);
    check("alpha `a b c` tail", "alpha\n`a b c` tail", 12);
    check("alpha *a b c* tail", "alpha\n*a b c* tail", 12);
    check("alpha ![a b](img) tail", "alpha\n![a b](img)\ntail", 12);
    check(
        "alpha [a b][id] tail\n\n[id]: /url\n",
        "alpha\n[a b][id]\ntail\n\n[id]: /url\n",
        12,
    );
    check(
        "alpha [averylonglink](url) tail",
        "alpha\n[averylonglink](url)\ntail",
        10,
    );
}

#[test]
fn default_width_wraps_prose_without_changing_inline_content() {
    let input =
        "Read [the *docs*](https://example.com \"title\") and ![image](img.png), `code here`.\n";
    let result = mdfmt::format(input, &FormatOptions::default()).unwrap();
    assert_eq!(
        result.output,
        "Read [the *docs*](https://example.com \"title\") and ![image](img.png),\n`code here`.\n"
    );
}

#[test]
fn preserves_bom_local_terminators_indentation_and_eof() {
    check(
        "\u{feff}alpha beta gamma\r\n",
        "\u{feff}alpha beta\r\ngamma\r\n",
        10,
    );
    check("alpha beta gamma\r", "alpha beta\rgamma\r", 10);
    check(
        "# Title\r\n\r\nalpha beta gamma\n",
        "# Title\r\n\r\nalpha beta\ngamma\n",
        10,
    );
    check(
        "# Title\r\n\r\nalpha beta gamma",
        "# Title\r\n\r\nalpha beta\r\ngamma",
        10,
    );
    check("  alpha beta gamma", "  alpha\nbeta gamma", 10);
    check("alpha\u{a0}beta gamma", "alpha\u{a0}beta\ngamma", 10);
}

#[test]
fn unsafe_and_out_of_scope_markdown_is_preserved() {
    for text in [
        "# alpha beta gamma\n",
        "```\nalpha beta gamma\n```\n",
        "| alpha beta gamma |\n| --- |\n| delta epsilon |\n",
        "    alpha beta gamma\n",
        "---\ntitle: alpha beta gamma\n---\n",
        "alpha\u{2028}beta gamma\n",
        "alpha beta gamma <!-- markdownlint-disable-line MD013 -->\n",
    ] {
        check(text, text, 10);
    }
}

#[test]
fn soft_breaks_are_joined_and_repacked_canonically() {
    check("alpha\nbeta gamma\n", "alpha beta\ngamma\n", 10);
    check("alpha\nbeta\n", "alpha beta\n", 20);
    check("alpha  beta\ngamma delta", "alpha  beta gamma\ndelta", 17);
    check("alpha\r\nbeta gamma\r\n", "alpha beta\r\ngamma\r\n", 10);
    check(
        "alpha\n[link text](url) tail",
        "alpha\n[link text](url)\ntail",
        15,
    );
}

#[test]
fn list_and_quote_prefixes_share_the_width_budget() {
    check(
        "- Parent\n  - This nested item has enough words to need wrapping.\n",
        "- Parent\n  - This nested item has\n    enough words to need\n    wrapping.\n",
        30,
    );
    check(
        "> This quotation has enough words to need wrapping.\n",
        "> This quotation has enough\n> words to need wrapping.\n",
        30,
    );
    check(
        "12. alpha beta gamma delta\n",
        "12. alpha beta\n    gamma delta\n",
        15,
    );
    check(
        "> - alpha beta gamma delta\n",
        "> - alpha beta\n>   gamma delta\n",
        15,
    );
    check(
        "- > alpha beta gamma delta\n",
        "- > alpha beta\n  > gamma delta\n",
        15,
    );
    check(
        "> > alpha beta gamma delta\n",
        "> > alpha beta\n> > gamma delta\n",
        15,
    );
    check(
        "- [x] alpha beta gamma delta\n",
        "- [x] alpha beta\n  gamma delta\n",
        16,
    );
}

#[test]
fn continuation_prefixes_and_lazy_lines_are_not_prose() {
    check(
        "- alpha\n  beta gamma delta\n",
        "- alpha beta\n  gamma delta\n",
        13,
    );
    check(
        "> alpha\n> beta gamma delta\n",
        "> alpha beta\n> gamma delta\n",
        13,
    );
    check(
        "> alpha\nbeta gamma delta\n",
        "> alpha beta\n> gamma delta\n",
        13,
    );
    check(
        "- alpha\nbeta gamma delta\n",
        "- alpha beta\n  gamma delta\n",
        13,
    );
    check(
        "- first\n\n  alpha beta gamma delta\n",
        "- first\n\n  alpha beta\n  gamma delta\n",
        13,
    );
    check("  alpha\n  beta gamma\n", "  alpha beta\ngamma\n", 12);
}

#[test]
fn a_bom_is_not_a_container_prefix_or_a_document_reflow_blocker() {
    check(
        "\u{feff}- alpha beta gamma delta\n\nalpha beta gamma delta\n",
        "\u{feff}- alpha\n  beta\n  gamma\n  delta\n\nalpha beta\ngamma\ndelta\n",
        10,
    );
    check(
        "\u{feff}> alpha beta gamma delta\n\nalpha beta gamma delta\n",
        "\u{feff}> alpha\n> beta\n> gamma\n> delta\n\nalpha beta\ngamma\ndelta\n",
        10,
    );
}

#[test]
fn hard_breaks_split_reflow_segments_without_being_changed() {
    check(
        "alpha beta gamma  \ndelta epsilon zeta\n",
        "alpha beta\ngamma  \ndelta\nepsilon\nzeta\n",
        10,
    );
    check(
        "alpha beta gamma\\\ndelta epsilon zeta\n",
        "alpha beta\ngamma\\\ndelta\nepsilon\nzeta\n",
        10,
    );
    check(
        "> alpha beta gamma  \n> delta epsilon zeta\n",
        "> alpha beta\n> gamma  \n> delta epsilon\n> zeta\n",
        15,
    );
}

#[test]
fn partial_permissions_and_protected_multiline_units_prevent_unsafe_joins() {
    for text in [
        "<!-- markdownlint-disable-next-line MD013 -->\nalpha\nbeta gamma delta\n",
        "alpha beta <!-- markdownlint-disable-line MD013 -->\ngamma delta\n",
        "alpha\tbeta gamma delta\n",
        "alpha [multi\nline](url) beta gamma\n",
    ] {
        check(text, text, 10);
    }
}

#[test]
fn wrapping_does_not_introduce_block_or_inline_syntax() {
    for text in [
        "alpha beta # heading",
        "alpha beta - item",
        "alpha beta 1. item",
        "alpha beta ---",
        "alpha beta > quote",
        "alpha beta ===",
        "alpha beta ~~~",
        "alpha beta <div> text",
    ] {
        check(text, text, 10);
    }
    check("alpha beta [id]: /url", "alpha beta\n[id]: /url", 10);
    check(
        "alpha beta | x |\n| --- |\n",
        "alpha beta\n| x | |\n--- |\n",
        10,
    );
}

#[test]
fn unsupported_math_and_directive_syntax_is_not_reflowed() {
    for text in [
        "$$ alpha beta gamma $$",
        "prefix $alpha beta gamma$ tail",
        ":::note alpha beta gamma",
        "::note[alpha beta gamma]",
        "prefix :note[alpha beta gamma] tail",
        "alpha beta :::note content",
    ] {
        check(text, text, 10);
    }
}

#[test]
fn raw_html_contents_are_not_reflowed() {
    for text in [
        "x <pre>alpha beta gamma</pre>",
        "x <script>alpha beta gamma</script>",
        "x <style>alpha beta gamma</style>",
        "x <textarea>alpha beta gamma</textarea>",
        "x <PRE class=code>alpha beta gamma</PRE>",
        "x <span style=\"white-space: pre\">alpha beta gamma</span>",
        "x <pre>\n\nalpha beta gamma delta\n\n</pre>",
    ] {
        check(text, text, 15);
    }
}

#[test]
fn configuration_and_fix_permissions_control_wrapping() {
    let text = "alpha beta gamma";
    let mut cfg = config(10);
    for (options, fix) in [
        (FormatOptions { fix: false }, true),
        (FormatOptions::default(), false),
    ] {
        cfg.fix = fix;
        assert_eq!(
            format_with_config(text, &options, &cfg).unwrap().output,
            text
        );
    }
    for rules in [
        json!({"MD013": false}),
        json!({"default": false}),
        json!({"line-length": {"enabled": false}}),
    ] {
        cfg.fix = true;
        cfg.rules = RuleConfig::from_value(rules).unwrap();
        assert_eq!(
            format_with_config(text, &FormatOptions::default(), &cfg)
                .unwrap()
                .output,
            text
        );
    }
    cfg.rules = RuleConfig::from_value(
        json!({"default": false, "line-length": {"line_length": 10, "severity": "warning"}}),
    )
    .unwrap();
    assert_eq!(
        format_with_config(text, &FormatOptions::default(), &cfg)
            .unwrap()
            .output,
        "alpha beta\ngamma"
    );
}

#[test]
fn inline_directives_remain_attached_to_their_source_targets() {
    check(
        "<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n",
        "<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n",
        10,
    );
    check(
        "<!-- markdownlint-disable-file MD013 -->\nalpha beta gamma\n",
        "<!-- markdownlint-disable-file MD013 -->\nalpha beta gamma\n",
        10,
    );
    check(
        "<!-- markdownlint-disable MD013 -->\nalpha beta gamma\n\n<!-- markdownlint-enable MD013 -->\nalpha beta gamma\n",
        "<!-- markdownlint-disable MD013 -->\nalpha beta gamma\n\n<!-- markdownlint-enable MD013 -->\nalpha beta\ngamma\n",
        10,
    );
    check(
        "alpha beta gamma\n\n<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n",
        "alpha beta\ngamma\n\n<!-- markdownlint-disable-next-line MD013 -->\nalpha beta gamma\n",
        10,
    );
    check(
        "<!-- markdownlint-configure-file {\"MD013\":{\"line_length\":7}} -->\nalpha beta gamma\n",
        "<!-- markdownlint-configure-file {\"MD013\":{\"line_length\":7}} -->\nalpha\nbeta\ngamma\n",
        80,
    );
}
