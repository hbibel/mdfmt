use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD030":options})).unwrap(),
        ..Default::default()
    }
}

fn fixed(input: &str, expected: &str, options: Value) {
    let settings = config(options);
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn list_padding_obeys_whole_list_single_and_multi_line_options() {
    for (input, expected, options) in [
        ("-  one\n-   two\n", "- one\n- two\n", json!(true)),
        ("1.  one\n3.   two\n", "1. one\n3. two\n", json!(true)),
        (
            "- one\n- two\n",
            "-  one\n-  two\n",
            json!({"ul_single":2,"ul_multi":4}),
        ),
        (
            "- one\n\n- two\n",
            "-   one\n\n-   two\n",
            json!({"ul_single":2,"ul_multi":3}),
        ),
        (
            "1. one\n2. two\n",
            "1.   one\n2.   two\n",
            json!({"ol_single":3,"ol_multi":4}),
        ),
        ("-   \n-  two\n", "-   \n- two\n", json!(true)),
        (
            "\u{feff}-  😀\r\n-   two\r\n",
            "\u{feff}- 😀\r\n- two\r\n",
            json!(true),
        ),
    ] {
        fixed(input, expected, options);
    }
}

#[test]
fn list_padding_diagnostics_match_pinned_ranges_and_option_selection() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md030-observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        if row["options"].get("ul_single") == Some(&json!(0)) {
            assert!(
                RuleConfig::from_value(json!({"default":false,"MD030":row["options"]})).is_err()
            );
            continue;
        }
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|finding|{
            assert_eq!(finding.code,"MD030");
            let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();
            json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),
                "length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Spaces after list markers [").unwrap().strip_suffix(']').unwrap(),
                "severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
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
fn code_tabs_and_invalid_interruptions_are_not_reinterpreted_as_padding() {
    for (input, options) in [
        ("-     literal\n-  two\n", json!({"ul_single":2})),
        ("-\tone\n- \ttwo\n", json!(true)),
        ("9.\ntext\n2.  a\n4.   b\n", json!(true)),
        ("    code\n2.  a\n4.   b\n1. c\n", json!(true)),
    ] {
        fixed(input, input, options);
    }
}

#[test]
fn padding_compounds_with_numbering_quotes_and_eof_in_original_coordinates() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD030":true,"MD029":true,"MD027":true,"MD047":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = ">  1.  one\n> 3.   two";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, "> 1. one\n> 2. two\n");
    assert!(first.diagnostics.is_empty());
    let mut actual = String::new();
    let mut cursor = 0;
    for edit in &first.edits {
        actual.push_str(&input[cursor..edit.span.start]);
        actual.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    actual.push_str(&input[cursor..]);
    assert_eq!(actual, first.output);
    assert!(
        format_with_config(&first.output, &FormatOptions::default(), &settings)
            .unwrap()
            .edits
            .is_empty()
    );
}

#[test]
fn aliases_permissions_and_disabled_reference_lines_are_respected() {
    fixed(
        "<!-- markdownlint-disable-next-line MD030 -->\n-  one\n-   two\n",
        "<!-- markdownlint-disable-next-line MD030 -->\n-  one\n- two\n",
        json!(true),
    );
    let settings = ResolvedConfig {
        fix: false,
        rules: RuleConfig::from_value(
            json!({"default":false,"list-marker-space":{"severity":"warning"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "-  one\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, input);
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn active_line_overrides_and_multi_line_reflow_do_not_change_padding_on_second_pass() {
    for (input, options) in [
        (
            "<!-- markdownlint-disable-next-line MD030 -->\n-  alpha beta gamma\n-   delta epsilon zeta",
            json!(true),
        ),
        (
            "- alpha beta gamma\n- delta epsilon zeta",
            json!({"ul_single":3,"ul_multi":1}),
        ),
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(
                json!({"default":false,"MD030":options,"MD013":{"line_length":12}}),
            )
            .unwrap(),
            ..Default::default()
        };
        let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        let second =
            format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(second.output, first.output, "{input:?}");
        assert!(second.edits.is_empty());
    }
}

#[test]
fn cli_checks_repairs_and_reports_preserved_padding_errors() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD030":true}}"#,
    )
    .unwrap();
    fs::write(&path, "-  one\r\n").unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), "-  one\r\n");
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "- one\r\n");
    let input = "-\tone\n- \ttwo\n";
    fs::write(&path, input).unwrap();
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    assert!(String::from_utf8_lossy(&write.stderr).contains("MD030"));
}

#[test]
fn zero_padded_one_is_not_a_valid_paragraph_interruption() {
    let input = "> - 01.\t\r\n>   01.item1\r\n>   01.  item2\r\n>   01.    \r\n>   01.\titem4\r\n";
    let settings = config(json!({"ul_single":1,"ol_single":4,"ul_multi":3,"ol_multi":2}));
    let result = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(result.diagnostics.len(), 1, "{:?}", result.diagnostics);
    assert_eq!(result.diagnostics[0].span, Some(2..4));
}

#[test]
fn oversized_padding_is_reported_without_allocating_a_replacement() {
    let settings = config(json!({"ul_single":u64::MAX}));
    let result = format_with_config("- one\n", &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, "- one\n");
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .message
            .contains("Expected: 18446744073709551615")
    );
}

#[test]
fn separator_blanks_do_not_make_a_single_line_list_multi_line() {
    fixed(
        "- one\n- two\n\nparagraph\n",
        "-  one\n-  two\n\nparagraph\n",
        json!({"ul_single":2,"ul_multi":3}),
    );
}

#[test]
fn interrupting_lists_carry_state_into_nested_empty_markers() {
    for prefix in ["text\n", "    code\n", "9.\ntext\n"] {
        let input = format!("{prefix}1. 1.\n    2)  alpha\n");
        let first =
            format_with_config(&input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(first.output, input);
        assert!(
            first.diagnostics.is_empty(),
            "{input:?}: {:?}",
            first.diagnostics
        );
    }
}
