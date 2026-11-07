use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(style: &str) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD003":{"style":style}})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn heading_styles_match_pinned_history_and_diagnostic_semantics() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md003-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let config = ResolvedConfig {
            rules: RuleConfig::from_value(json!({"default":false,"MD003":row["options"]})).unwrap(),
            ..Default::default()
        };
        let result = format_with_config(input, &FormatOptions { fix: false }, &config).unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|f|json!({"line":source.position(f.span.as_ref().unwrap().start).unwrap().line,"detail":f.message.strip_prefix("Heading style [").unwrap().strip_suffix(']').unwrap(),"severity":if f.severity==Severity::Warning{"warning"}else{"error"}})).collect();
        if actual != *row["findings"].as_array().unwrap() {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn safe_delimiter_conversions_preserve_content_and_original_edit_coordinates() {
    for (style, input, expected) in [
        ("atx", "Title\n===\n", "# Title\n"),
        ("atx_closed", "# Title\n", "# Title #\n"),
        ("setext", "# Title\n", "Title\n===\n"),
        ("setext_with_atx_closed", "### Third\n", "### Third ###\n"),
        ("atx", "> Title\n> ===\n", "> # Title\n"),
        ("setext", "> # Title\n", "> Title\n> ===\n"),
        ("atx_closed", "# Title#\n", "# Title# #\n"),
        ("atx", "# Title #\n", "# Title\n"),
        ("setext", "\u{feff}# 😀\r\n", "\u{feff}😀\r\n===\r\n"),
        (
            "atx_closed",
            "# `code` [link](url)\n",
            "# `code` [link](url) #\n",
        ),
        ("atx", "- Title\n  ===\n", "- # Title\n"),
    ] {
        let config = config(style);
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected, "{style}: {input:?}");
        assert!(first.diagnostics.is_empty());
        let mut reconstructed = input.to_owned();
        for edit in first.edits.iter().rev() {
            assert_eq!(edit.origin, "MD003");
            reconstructed.replace_range(edit.span.clone(), &edit.replacement);
        }
        assert_eq!(reconstructed, expected);
        let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
        assert_eq!(second.output, first.output);
        assert!(second.edits.is_empty());
    }
}

#[test]
fn unsafe_or_disabled_style_changes_remain_diagnostics_without_meaning_changes() {
    for (style, input) in [
        ("setext", "### Third\n"),
        ("setext", "Paragraph\n# Title\n"),
        ("atx", "A\nB\n===\n"),
        ("atx", "A  \nB\n===\n"),
        ("atx", "> A  \n> B\n> ===\n"),
        ("atx", "A `code\nspan`\n===\n"),
        ("setext", "#\n"),
    ] {
        let first = format_with_config(input, &FormatOptions::default(), &config(style)).unwrap();
        assert_eq!(first.output, input, "{style}: {input:?}");
        assert_eq!(first.diagnostics.len(), 1);
        assert_eq!(first.diagnostics[0].code, "MD003");
    }
    let input = "<!-- markdownlint-disable-file MD003 -->\n# Title\n";
    let result = format_with_config(input, &FormatOptions::default(), &config("setext")).unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.is_empty());
    let input = "<!-- markdownlint-disable-next-line MD013 -->\n# Title\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD003":{"style":"setext"},"MD013":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>(),
        vec!["MD003"]
    );
}

#[test]
fn consistent_style_uses_disabled_reference_headings() {
    let input = "<!-- markdownlint-disable-next-line MD003 -->\n# First #\n# Second\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config("consistent")).unwrap();
    assert_eq!(
        result.output,
        "<!-- markdownlint-disable-next-line MD003 -->\n# First #\n# Second #\n"
    );
}

#[test]
fn one_unsafe_heading_does_not_block_independent_safe_style_repairs() {
    let input = "Paragraph\n# Unsafe\n\n# Safe\n";
    let result = format_with_config(input, &FormatOptions::default(), &config("setext")).unwrap();
    assert_eq!(result.output, "Paragraph\n# Unsafe\n\nSafe\n===\n");
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn empty_closed_headings_do_not_require_a_second_spacing_fix() {
    let input = "#\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD003":{"style":"atx_closed"},"MD021":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "# #\n");
    assert_eq!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        first.output
    );
}

#[test]
fn table_separators_unlock_heading_styles_in_the_same_invocation() {
    let input = "| a |\n| --- |\n| b |\n# Title\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD003":{"style":"setext"},"MD058":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, "| a |\n| --- |\n| b |\n\nTitle\n===\n");
    assert_eq!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .output,
        first.output
    );
}

#[test]
fn literal_hashes_are_not_lost_when_native_heading_ranges_are_ambiguous() {
    for (style, input) in [
        ("atx_closed", "# # Title\n"),
        ("atx_closed", "### # Title\n"),
        ("atx_closed", "> # # Title\n"),
        ("atx_closed", "- # # Title\n"),
        ("atx", "# Title # #\n"),
        ("atx", "## Title # #\n"),
        ("atx", "Title #\n===\n"),
    ] {
        let result = format_with_config(input, &FormatOptions::default(), &config(style)).unwrap();
        assert_eq!(result.output, input, "{style}: {input:?}");
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            vec!["MD003"]
        );
    }
}

#[test]
fn literal_hash_preservation_does_not_expose_punctuation_to_a_second_pass() {
    let input = "# Title. # #\n";
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD003":{"style":"atx"},"MD026":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert_eq!(second.diagnostics, first.diagnostics);
}

#[test]
fn ambiguous_style_boundaries_do_not_block_approved_punctuation_fixes() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD003":{"style":"atx_closed"},"MD026":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config("# # Title!\n", &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "# # Title\n");
    assert!(result.edits.iter().all(|edit| edit.origin == "MD026"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>(),
        vec!["MD003"]
    );
}

#[test]
fn cli_style_repairs_and_preserved_violations_have_distinct_exit_status() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    fs::write(&path, "Title\n===\n").unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD003":{"style":"atx"}}}"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Title\n===\n");
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "# Title\n");
    fs::write(&path, "### Third\n").unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"fix":false,"config":{"default":false,"MD003":{"style":"setext","severity":"warning"}}}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .arg("doc.md")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"### Third\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning MD003"));
}
