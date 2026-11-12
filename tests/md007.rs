use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD007":options})).unwrap(),
        ..Default::default()
    }
}
fn fixed(input: &str, expected: &str, settings: &ResolvedConfig) {
    let first = format_with_config(input, &FormatOptions::default(), settings).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let second = format_with_config(&first.output, &FormatOptions::default(), settings).unwrap();
    assert_eq!(second.output, first.output);
    assert_eq!(second.diagnostics, first.diagnostics);
    assert!(second.edits.is_empty());
}

#[test]
fn nested_unordered_indentation_preserves_ownership() {
    let input = "- a\n  - b\n";
    let result = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"indent":4})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(4..8));
    assert!(
        result.diagnostics[0]
            .message
            .ends_with("[Expected: 4; Actual: 2]")
    );
    fixed(input, "- a\n    - b\n", &config(json!({"indent":4})));
}

#[test]
fn root_start_indent_and_quote_adjustments_are_physical() {
    for (input, expected, options) in [
        ("  - a\n", "- a\n", json!(true)),
        ("- a\n", "  - a\n", json!({"start_indented":true})),
        (
            "> - a\n>   - b\n",
            "> - a\n>     - b\n",
            json!({"indent":4}),
        ),
    ] {
        fixed(input, expected, &config(options));
    }
    let input = "> - a\n>   - b\n";
    let result = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"indent":4})),
    )
    .unwrap();
    assert_eq!(result.diagnostics[0].span, Some(6..12));
}

#[test]
fn unordered_nesting_ranges_options_and_scopes_match_pinned_source() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md007-observations.json"
    ))
    .unwrap();
    let mut errors = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|finding|{assert_eq!(finding.code,"MD007");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),"length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Unordered list indentation [").unwrap().strip_suffix(']').unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        if actual != *row["findings"].as_array().unwrap() {
            errors.push(format!(
                "{} {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn ordered_ancestors_reset_at_quotes_and_footnotes_add_their_base() {
    for input in ["1. a\n   - b\n     - c\n", "- a\n  1. b\n     - c\n"] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"indent":4})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
    let result = format_with_config(
        "1. > - a\n   >   - b\n",
        &FormatOptions { fix: false },
        &config(json!({"indent":4})),
    )
    .unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .message
            .ends_with("[Expected: 4; Actual: 2]")
    );
    let result = format_with_config(
        "[^a]:\n    - a\n      - b\n",
        &FormatOptions { fix: false },
        &config(json!({"indent":4})),
    )
    .unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .message
            .ends_with("[Expected: 8; Actual: 6]")
    );
}

#[test]
fn nesting_repairs_do_not_change_code_tables_definitions_or_links() {
    for input in [
        "  - a\n\n        code\n",
        "  - a\n\n    | a |\n    | - |\n    | b |\n",
        "  - a\n\n    [id]: /url\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        fixed(input, input, &config(json!(true)));
    }
    fixed("  - [a](url)\n", "- [a](url)\n", &config(json!(true)));
}

#[test]
fn mixed_rule_permissions_keep_disabled_anchor_bytes_without_oscillation() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD005":true,"MD007":true})).unwrap(),
        ..Default::default()
    };
    fixed(" - a\n- b\n", "- a\n- b\n", &settings);
    let input = " - a <!-- markdownlint-disable-line MD007 -->\n- b\n";
    fixed(input, input, &settings);
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD005");
    let input = "<!-- markdownlint-disable-file MD007 -->\n- a\n - b\n";
    fixed(
        input,
        "<!-- markdownlint-disable-file MD007 -->\n- a\n- b\n",
        &settings,
    );
}

#[test]
fn huge_targets_report_exact_expectations_without_allocating_padding() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let input = "- a\n  - b\n";
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    fs::write(&path, input).unwrap();
    fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"config":{"default":false,"ul-indent":{"indent":u64::MAX,"start_indented":true,"start_indent":u64::MAX}}}).to_string()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(1));
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("huge indentation target failed to terminate");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), input);
    let errors = fs::read_to_string(errors).unwrap();
    assert_eq!(errors.matches("Error MD007").count(), 2);
    assert!(errors.contains("Expected: 18446744073709551615; Actual: 0"));
    assert!(errors.contains("Expected: 36893488147419103230; Actual: 2"));
}

#[test]
fn quote_spacing_conflicts_terminate_and_leave_a_stable_remaining_finding() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":{"default":false,"MD007":{"start_indented":true},"MD027":true}})
            .to_string(),
    )
    .unwrap();
    for input in ["> - a\n", ">   - a\n"] {
        fs::write(&path, input).unwrap();
        for _ in 0..2 {
            let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args(["--write", "doc.md"])
                .stdout(Stdio::null())
                .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
                .spawn()
                .unwrap();
            let start = Instant::now();
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert_eq!(status.code(), Some(1));
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("MD007 and MD027 indentation conflict did not terminate");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), "> - a\n");
            let errors = fs::read_to_string(&errors).unwrap();
            assert_eq!(errors.matches("Error MD007:").count(), 1);
            assert!(!errors.contains("Error MD027:"));
            assert_eq!(errors.matches("Error rule-conflict:").count(), 1);
            assert!(errors.contains("MD007, MD027; retained MD027 priority"));
        }
    }
}

#[test]
fn quote_conflict_gate_respects_disabled_and_list_item_permissions() {
    for rules in [
        json!({"default":false,"MD007":{"start_indented":true},"MD027":false}),
        json!({"default":false,"MD007":{"start_indented":true},"MD027":{"list_items":false}}),
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(rules).unwrap(),
            ..Default::default()
        };
        fixed("> - a\n", ">   - a\n", &settings);
    }
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD007":{"start_indented":true},"MD027":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    fixed(
        "<!-- markdownlint-disable-next-line MD027 -->\n> - a\n",
        "<!-- markdownlint-disable-next-line MD027 -->\n>   - a\n",
        &settings,
    );
}

#[test]
fn indentation_composes_with_numbering_padding_reflow_and_heading_punctuation() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD005":true,"MD007":{"indent":4},"MD029":true,"MD030":true,"MD013":{"line_length":30},"MD026":true})).unwrap(),..Default::default()};
    let input = "  - # Heading!\n  - a paragraph with enough words to need wrapping [link](url)\n\n1.  one\n3.   two\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(
        first
            .output
            .starts_with("- # Heading\n- a paragraph with enough\n"),
        "{}",
        first.output
    );
    assert!(first.output.ends_with("1. one\n2. two\n"));
    assert!(first.output.contains("[link](url)"));
    assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
    fixed(&first.output, &first.output, &settings);
    let mut reconstructed = String::new();
    let mut cursor = 0;
    for edit in first.edits {
        reconstructed.push_str(&input[cursor..edit.span.start]);
        reconstructed.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    reconstructed.push_str(&input[cursor..]);
    assert_eq!(reconstructed, first.output);
}

#[test]
fn ul_indent_cli_warning_and_fix_controls_apply_in_both_modes() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    for fix in [false, true] {
        let input = "\u{feff}  - 😀\r\n";
        fs::write(&path, input).unwrap();
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"fix":fix,"config":{"default":false,"ul-indent":{"severity":"warning"}}})
                .to_string(),
        )
        .unwrap();
        let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args(["--check", "doc.md"])
            .output()
            .unwrap();
        assert_eq!(check.status.code(), Some(i32::from(fix)));
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        for _ in 0..2 {
            let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args(["--write", "doc.md"])
                .output()
                .unwrap();
            assert_eq!(write.status.code(), Some(0));
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                if fix { "\u{feff}- 😀\r\n" } else { input }
            );
            assert_eq!(
                String::from_utf8_lossy(&write.stderr)
                    .matches("Warning MD007")
                    .count(),
                usize::from(!fix)
            );
        }
    }
}

#[test]
fn dense_and_deep_unordered_lists_keep_all_findings_with_bounded_work() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let input = format!(
        "{}\n- root\n{}",
        "  - item\n".repeat(1000),
        (1..50)
            .map(|depth| format!("{}- item\n", "  ".repeat(depth)))
            .collect::<String>()
    );
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    fs::write(&path, &input).unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"fix":false,"config":{"default":false,"MD007":{"indent":4,"severity":"warning"}}})
            .to_string(),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(0));
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("dense/deep unordered indentation exceeded work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), input);
    assert_eq!(
        fs::read_to_string(errors)
            .unwrap()
            .matches("Warning MD007")
            .count(),
        1049
    );
}
