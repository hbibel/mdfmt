use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD027":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn incidental_quote_separator_spaces_are_removed_without_reparenting_lists() {
    for (input, expected) in [
        (">  text\n", "> text\n"),
        (">  >  text\n", "> > text\n"),
        ("- >  item\n", "- > item\n"),
        (
            "> - text\n>    continuation\n",
            "> - text\n>    continuation\n",
        ),
        (">   \n", "> \n"),
        ("\u{feff}>  😀 text\r\n", "\u{feff}> 😀 text\r\n"),
    ] {
        let config = config(json!(true));
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, expected, "{input:?}");
        assert!(
            format_with_config(&first.output, &FormatOptions::default(), &config)
                .unwrap()
                .edits
                .is_empty()
        );
    }
}

#[test]
fn quote_prefix_diagnostics_match_pinned_token_and_list_exclusions() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md027-observations.json"
    ))
    .unwrap();
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("error").is_none())
    {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual: Vec<_> = result.diagnostics.iter().map(|finding| {
            assert_eq!(finding.code, "MD027");
            let span = finding.span.clone().unwrap();
            let position = source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16 - usize::from(position.line == 1 && input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"context":finding.message.strip_prefix("Multiple spaces after blockquote symbol [Context: \"").unwrap().strip_suffix("\"]").unwrap(),
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
fn protected_source_and_uncertain_tabbed_prefixes_retain_configured_findings() {
    for input in [
        ">  # Heading\n",
        ">  ```\n>   code\n>  ```\n",
        ">  | a |\n> | - |\n> | b |\n",
        ">\t text\n",
    ] {
        let config = config(json!(true));
        let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(first.output, input);
        assert!(!first.diagnostics.is_empty());
    }
}

#[test]
fn aliases_warning_permissions_and_active_line_overrides_are_respected() {
    let mut config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"no-multiple-space-blockquote":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    config.fix = false;
    let input = ">  text\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD027":true,"MD013":{"line_length":10}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "<!-- markdownlint-disable-next-line MD027 -->\n>  alpha beta gamma";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn quote_deletions_compose_with_whitespace_heading_fence_and_eof_stages() {
    let config = ResolvedConfig {rules:RuleConfig::from_value(json!({"default":false,"MD027":true,"MD009":true,"MD010":true,"MD023":true,"MD048":{"style":"backtick"},"MD047":true})).unwrap(), ..Default::default()};
    let input = ">   alpha \n>\n~~~js\ncode\n~~~";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, "> alpha\n>\n```js\ncode\n```\n");
    assert!(result.diagnostics.is_empty());
    let mut applied = String::new();
    let mut cursor = 0;
    for edit in &result.edits {
        applied.push_str(&input[cursor..edit.span.start]);
        applied.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    applied.push_str(&input[cursor..]);
    assert_eq!(applied, result.output);
    assert!(
        format_with_config(&result.output, &FormatOptions::default(), &config)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn later_tabbed_quote_prefixes_block_unrelated_edits_too() {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD026":true,"MD027":true})).unwrap(),
        ..Default::default()
    };
    let input = "# Heading!\n\n> text\n>\t more\n";
    let first = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    assert!(
        first
            .diagnostics
            .iter()
            .any(|finding| finding.code == "MD026")
    );
}

#[test]
fn cli_repairs_prose_quote_spacing_and_preserves_literal_code() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let configuration = dir.path().join(".markdownlint-cli2.jsonc");
    fs::write(&path, ">  prose\r\n").unwrap();
    fs::write(
        &configuration,
        r#"{"config":{"default":false,"MD027":true}}"#,
    )
    .unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), ">  prose\r\n");
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "> prose\r\n");
    let input = ">  ```\n>   code\n>  ```\n";
    fs::write(&path, input).unwrap();
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    assert!(String::from_utf8_lossy(&write.stderr).contains("MD027"));
}

#[test]
fn virtual_tab_prefixes_do_not_create_invalid_zero_length_findings() {
    let input = ">  ~~~\n>\t code\n> ~~~\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].span, Some(2..3));
}

#[test]
fn each_quote_prefix_has_its_own_list_and_literal_ownership() {
    let input = ">  > - text\n";
    let settings = config(json!({"list_items":false}));
    let diagnostic = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(diagnostic.diagnostics.len(), 1);
    assert_eq!(diagnostic.diagnostics[0].span, Some(2..3));
    assert_eq!(
        format_with_config(input, &FormatOptions::default(), &settings)
            .unwrap()
            .output,
        "> > - text\n"
    );
}

#[test]
fn lazy_literal_interiors_do_not_gain_quote_prefix_findings() {
    let mut failures = vec![];
    for (input, spans) in [
        ("> > `a\n>   b`\n", vec![]),
        ("> > <div>\n>   >  more\n", vec![]),
        ("> > text\n>     >  more\n", vec![(11, 15)]),
    ] {
        let first =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        let actual: Vec<_> = first
            .diagnostics
            .iter()
            .map(|finding| {
                let span = finding.span.clone().unwrap();
                (span.start, span.end)
            })
            .collect();
        if actual != spans {
            failures.push(format!("{input:?}: got {actual:?}, expected {spans:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
