use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD005":options})).unwrap(),
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
fn sibling_indentation_reports_and_repairs_only_preserved_structure() {
    let input = "- a\n - b\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(4..7));
    assert!(
        result.diagnostics[0]
            .message
            .ends_with("[Expected: 0; Actual: 1]")
    );
    fixed(input, "- a\n- b\n", &config(json!(true)));
    fixed(
        "\u{feff}- 😀\r\n - b\r\n",
        "\u{feff}- 😀\r\n- b\r\n",
        &config(json!(true)),
    );
}

#[test]
fn ordered_alignment_is_not_numbering() {
    let input = "1. a\n 2. b\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(5..9));
    fixed(input, "1. a\n2. b\n", &config(json!(true)));
    fixed(
        " 9. a\n10. b\n11. c\n",
        " 9. a\n10. b\n11. c\n",
        &config(json!(true)),
    );
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD005":true,"MD029":true,"MD030":true}),
        )
        .unwrap(),
        ..Default::default()
    };
    fixed("1.  a\n 3.   b\n", "1. a\n2. b\n", &settings);
}

#[test]
fn sibling_indentation_diagnostics_match_pinned_source_ranges() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md005-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|finding|{assert_eq!(finding.code,"MD005");let span=finding.span.clone().unwrap();let position=source.position(span.start).unwrap();json!({"line":position.line,"column":position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),"length":mdfmt::utf16_len(&input[span]),"detail":finding.message.strip_prefix("Inconsistent indentation for list items at the same level [").unwrap().strip_suffix(']').unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
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
fn indentation_preserves_code_tables_definitions_and_disabled_anchors() {
    for input in [
        "- a\n - b\n\n       code\n",
        "- a\n - b\n\n   | a |\n   | - |\n   | b |\n",
        "- a\n - b\n\n   [id]: /url\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        fixed(input, input, &config(json!(true)));
    }
    let input = "<!-- markdownlint-disable-next-line MD005 -->\n - a\n- b\n";
    fixed(
        input,
        "<!-- markdownlint-disable-next-line MD005 -->\n - a\n - b\n",
        &config(json!(true)),
    );
    let input = "- a\n - b <!-- markdownlint-disable-line list-indent -->\n";
    fixed(input, input, &config(json!(true)));
}

#[test]
fn sibling_repairs_delegate_unordered_lists_to_enabled_nesting_rule() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD005":true,"MD007":true})).unwrap(),
        ..Default::default()
    };
    let input = " - a <!-- markdownlint-disable-line MD007 -->\n- b\n";
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.iter().any(|d| d.code == "MD005"));
}

#[test]
fn list_indent_alias_cli_fix_modes_and_warning_exit_codes_are_consistent() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    for (fix, expected, remaining) in [(false, "- a\r\n - b\r\n", 1), (true, "- a\r\n- b\r\n", 0)] {
        for severity in ["error", "warning"] {
            let input = "- a\r\n - b\r\n";
            fs::write(&path, input).unwrap();
            fs::write(
                dir.path().join(".markdownlint-cli2.jsonc"),
                json!({"fix":fix,"config":{"default":false,"list-indent":{"severity":severity}}})
                    .to_string(),
            )
            .unwrap();
            let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args(["--check", "doc.md"])
                .output()
                .unwrap();
            assert_eq!(
                check.status.code(),
                Some(i32::from(fix || severity == "error"))
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), input);
            for _ in 0..2 {
                let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(dir.path())
                    .args(["--write", "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(
                    write.status.code(),
                    Some(i32::from(remaining > 0 && severity == "error"))
                );
                assert_eq!(
                    String::from_utf8_lossy(&write.stderr)
                        .matches("MD005")
                        .count(),
                    remaining
                );
                assert_eq!(fs::read_to_string(&path).unwrap(), expected);
            }
        }
    }
}

#[test]
fn dense_sibling_lists_produce_all_findings_within_a_bounded_workload() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let input = format!("- first\n{}", " - item\n".repeat(1000));
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    fs::write(&path, &input).unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"fix":false,"config":{"default":false,"MD005":{"severity":"warning"}}}).to_string(),
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
            panic!("dense sibling indentation exceeded work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), input);
    assert_eq!(
        fs::read_to_string(errors)
            .unwrap()
            .matches("Warning MD005")
            .count(),
        1000
    );
}

#[test]
fn disabled_quote_spacing_anchors_do_not_cause_sibling_repair_cycles() {
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
        json!({"config":{"default":false,"MD005":true,"MD027":true}}).to_string(),
    )
    .unwrap();
    for input in [
        ">  1. a <!-- markdownlint-disable-line MD027 -->\n> 2. b\n",
        ">  - a <!-- markdownlint-disable-line MD027 -->\n> - b\n",
    ] {
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
                    panic!("MD005 and a disabled MD027 anchor did not terminate");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), input);
            let errors = fs::read_to_string(&errors).unwrap();
            assert_eq!(errors.matches("Error MD005").count(), 1);
            assert!(!errors.contains("Error MD027"));
        }
    }
}
