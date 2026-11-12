use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(rules: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    }
}

fn explanations(result: &mdfmt::FormatResult) -> Vec<&mdfmt::Diagnostic> {
    result
        .diagnostics
        .iter()
        .filter(|finding| finding.code == "rule-conflict")
        .collect()
}

#[test]
fn opposed_indentation_reports_the_retained_rule_priority() {
    for (input, expected, rules, losing, retained, count) in [
        (
            "> - a\n",
            "> - a\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD027":true}),
            "MD007",
            "MD027",
            1,
        ),
        (
            "- # H\n",
            "- # H\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD023":true}),
            "MD007",
            "MD023",
            1,
        ),
        (
            " - # A <!-- markdownlint-disable-line MD023 -->\n- # B\n",
            " - # A <!-- markdownlint-disable-line MD023 -->\n- # B\n",
            json!({"default":false,"MD005":true,"MD023":true}),
            "MD005",
            "MD023",
            1,
        ),
        (
            "- - a\n",
            "- - a\n",
            json!({"default":false,"MD007":{"indent":4},"MD030":true}),
            "MD007",
            "MD030",
            1,
        ),
        (
            "- - a\n",
            "-   -   a\n",
            json!({"default":false,"MD007":true,"MD030":{"ul_single":3}}),
            "MD007",
            "MD030",
            1,
        ),
    ] {
        for input in [
            input.to_owned(),
            format!("\u{feff}{}", input.replace('\n', "\r\n")),
            input.replace('\n', "\r"),
        ] {
            let settings = config(rules.clone());
            let first = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
            let expected = if input.starts_with('\u{feff}') {
                format!("\u{feff}{}", expected.replace('\n', "\r\n"))
            } else if input.contains('\r') {
                expected.replace('\n', "\r")
            } else {
                expected.into()
            };
            assert_eq!(first.output, expected);
            let conflicts = explanations(&first);
            assert_eq!(conflicts.len(), count, "{input:?}: {:?}", first.diagnostics);
            assert!(
                first
                    .diagnostics
                    .iter()
                    .any(|finding| finding.code == losing)
            );
            for conflict in conflicts {
                assert_eq!(conflict.rule_id, None);
                assert!(conflict.message.contains(losing));
                assert!(conflict.message.contains(&format!("retained {retained}")));
                let span = conflict.span.clone().unwrap();
                assert!(first.output.is_char_boundary(span.start));
                assert!(span.end <= first.output.len());
            }
            let again =
                format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
            assert_eq!(again.output, first.output);
            assert_eq!(again.diagnostics, first.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
}

#[test]
fn disabled_opponents_and_unsafe_repairs_do_not_manufacture_conflicts() {
    for (input, expected, rules) in [
        (
            "- # H\n",
            "  - # H\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD023":false}),
        ),
        (
            "- # H <!-- markdownlint-disable-line MD023 -->\n",
            "  - # H <!-- markdownlint-disable-line MD023 -->\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD023":true}),
        ),
        (
            "> - a\n",
            ">   - a\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD027":{"list_items":false}}),
        ),
        (
            "- - a\n",
            "-   - a\n",
            json!({"default":false,"MD007":{"indent":4},"MD030":false}),
        ),
        (
            "- a\n",
            "- a\n",
            json!({"default":false,"MD007":{"start_indented":true,"start_indent":16000},"MD023":true}),
        ),
        (
            "# Heading\n\n[x](#missing)\n",
            "# Heading\n\n[x](#missing)\n",
            json!({"default":false,"MD051":true,"MD043":{"headings":["# Required"]}}),
        ),
        (
            "- item\n\n      code\n",
            "- item\n\n      code\n",
            json!({"default":false,"MD007":{"start_indented":true},"MD023":true}),
        ),
        (
            "- a\n  - | a |\n    | - |\n    | b |\n",
            "- a\n  - | a |\n    | - |\n    | b |\n",
            json!({"default":false,"MD007":{"indent":4},"MD030":true}),
        ),
    ] {
        let settings = config(rules);
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, expected);
        assert!(
            explanations(&result).is_empty(),
            "{input:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn blank_separator_budget_explains_only_actual_conflicts() {
    let rules =
        json!({"default":false,"MD012":{"maximum":1},"MD022":{"lines_above":2,"lines_below":2}});
    for (input, expected) in [
        ("one\n# Heading\ntext\n", "one\n\n# Heading\n\ntext\n"),
        ("one\n\n# Heading\n\ntext\n", "one\n\n# Heading\n\ntext\n"),
    ] {
        for input in [
            input.to_owned(),
            format!("\u{feff}{}", input.replace('\n', "\r\n")),
            input.replace('\n', "\r"),
        ] {
            let settings = config(rules.clone());
            let result = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
            let expected = if input.starts_with('\u{feff}') {
                format!("\u{feff}{}", expected.replace('\n', "\r\n"))
            } else if input.contains('\r') {
                expected.replace('\n', "\r")
            } else {
                expected.into()
            };
            assert_eq!(result.output, expected);
            assert_eq!(
                result
                    .diagnostics
                    .iter()
                    .filter(|d| d.code == "MD022")
                    .count(),
                2
            );
            let conflicts = explanations(&result);
            assert_eq!(conflicts.len(), 2, "{:?}", result.diagnostics);
            assert!(
                conflicts
                    .iter()
                    .all(|d| d.message.contains("MD022") && d.message.contains("retained MD012"))
            );
            let again =
                format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
            assert_eq!(again.output, result.output);
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
            for options in [FormatOptions { fix: false }, FormatOptions { fix: true }] {
                let settings = ResolvedConfig {
                    fix: false,
                    ..settings.clone()
                };
                let nofix = format_with_config(&result.output, &options, &settings).unwrap();
                assert_eq!(nofix.output, result.output);
                assert_eq!(nofix.diagnostics, result.diagnostics);
                assert!(nofix.edits.is_empty());
            }
        }
    }
    for (input, rules) in [
        ("one\ntext\n", rules.clone()),
        (
            "one\n# Heading\ntext\n",
            json!({"default":false,"MD012":true,"MD022":false}),
        ),
        (
            "one\n# Heading\ntext\n",
            json!({"default":false,"MD012":{"maximum":2},"MD022":{"lines_above":2,"lines_below":2}}),
        ),
        ("> one\n> # Heading\n> text\n", rules.clone()),
        (
            "one\n<!-- markdownlint-disable-next-line MD022 -->\n# Heading\ntext\n",
            rules.clone(),
        ),
        (
            "one\n# Heading\ntext\n",
            json!({"default":false,"MD012":false,"MD022":{"lines_above":2,"lines_below":2}}),
        ),
        (
            "one\n# Heading\ntext\n",
            json!({"default":false,"MD012":false,"MD022":{"lines_above":1000000000,"lines_below":1000000000}}),
        ),
    ] {
        let result = format_with_config(input, &FormatOptions::default(), &config(rules)).unwrap();
        assert!(
            explanations(&result).is_empty(),
            "{input:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn conflict_cli_preserves_warning_exits_and_both_fix_controls() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let input = "> - a\n";
    for severity in ["warning", "error"] {
        for fix in [false, true] {
            fs::write(&path, input).unwrap();
            fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"ul-indent":{"start_indented":true,"severity":severity},"MD027":{"severity":"warning"}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                for _ in 0..2 {
                    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                        .current_dir(dir.path())
                        .args([mode, "doc.md"])
                        .output()
                        .unwrap();
                    assert_eq!(result.status.code(), Some(i32::from(severity == "error")));
                    let errors = String::from_utf8_lossy(&result.stderr);
                    assert_eq!(errors.matches("rule-conflict").count(), 1, "{errors}");
                    assert_eq!(fs::read_to_string(&path).unwrap(), input);
                }
            }
        }
    }
}

#[test]
fn partial_blank_line_directives_and_comment_boundaries_keep_local_conflicts() {
    let settings = config(
        json!({"default":false,"MD012":{"maximum":1,"severity":"warning"},"MD022":{"lines_above":2,"lines_below":2,"severity":"warning"}}),
    );
    for input in [
        "one\n<!-- note -->\n# Heading\ntext\n",
        "one\n# Heading\n<!-- markdownlint-disable-next-line MD012 -->\ntext\n",
        "one\n<!-- markdownlint-disable-next-line MD012 -->\n\n# Heading\ntext\n",
    ] {
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        let conflicts = explanations(&result);
        assert_eq!(conflicts.len(), 1, "{input:?}: {:?}", result.diagnostics);
        assert_eq!(conflicts[0].severity, mdfmt::Severity::Warning);
        let again =
            format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, result.output);
        assert_eq!(again.diagnostics, result.diagnostics);
        assert!(again.edits.is_empty());
    }
}

#[test]
fn dense_conflict_explanations_have_exact_counts_and_a_bounded_cli_budget() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    for (input, rules, count, ordinary) in [
        (
            "- # H\n".repeat(400),
            json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD023":{"severity":"warning"}}),
            400,
            "MD007",
        ),
        (
            "- # H\n".repeat(400),
            json!({"default":false,"MD007":false,"MD023":{"severity":"warning"}}),
            0,
            "MD007",
        ),
        (
            (0..200)
                .map(|index| format!("paragraph {index}\n\n# Heading {index}\n\ntext {index}\n\n"))
                .collect::<String>()
                .trim_end_matches('\n')
                .to_owned()
                + "\n",
            json!({"default":false,"MD012":{"severity":"warning"},"MD022":{"lines_above":2,"lines_below":2,"severity":"warning"}}),
            400,
            "MD022",
        ),
    ] {
        fs::write(&path, &input).unwrap();
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"config":rules}).to_string(),
        )
        .unwrap();
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
                    assert_eq!(status.code(), Some(0));
                    break;
                }
                if start.elapsed() > Duration::from_secs(30) {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("conflict analysis exceeded work budget");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), input);
            let errors = fs::read_to_string(&errors).unwrap();
            assert_eq!(errors.matches("rule-conflict").count(), count);
            assert_eq!(
                errors.matches(&format!("Warning {ordinary}:")).count(),
                count
            );
        }
    }
}

#[test]
fn complete_pipeline_composes_original_ranges_and_settles_across_rule_families() {
    for (input, expected, rules) in [
        (
            "# Heading!\n\nA paragraph with enough words to wrap before this [go](#Heading).\n",
            None,
            json!({"default":false,"MD026":true,"MD003":{"style":"setext"},"MD013":{"line_length":32},"MD051":true}),
        ),
        (
            ">  1.  one\n> 3.   two\n\n- a\n  - b\n",
            Some("> 1. one\n> 2. two\n\n- a\n    - b\n"),
            json!({"default":false,"MD027":true,"MD029":true,"MD030":true,"MD005":true,"MD007":{"indent":4}}),
        ),
        (
            "# Heading!\n*one* **two** [go](url)",
            Some("# Heading\n\n_one_ __two__ [go](url)\n"),
            json!({"default":false,"MD026":true,"MD049":{"style":"underscore"},"MD050":{"style":"underscore"},"MD022":true,"MD012":true,"MD013":{"line_length":32},"MD047":true}),
        ),
    ] {
        for bom in [false, true] {
            let input = if bom {
                format!("\u{feff}{}", input.replace('\n', "\r\n"))
            } else {
                input.into()
            };
            let settings = config(rules.clone());
            let first = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
            if let Some(expected) = expected {
                let expected = if bom {
                    format!("\u{feff}{}", expected.replace('\n', "\r\n"))
                } else {
                    expected.into()
                };
                assert_eq!(first.output, expected);
            } else {
                assert!(first.output.contains("[go](#heading)"));
                assert!(!first.output.contains("Heading!"));
            }
            assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
            let second =
                format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
            assert_eq!(second.output, first.output);
            assert!(second.edits.is_empty());
            let mut actual = String::new();
            let mut cursor = 0;
            for edit in &first.edits {
                actual.push_str(&input[cursor..edit.span.start]);
                actual.push_str(&edit.replacement);
                cursor = edit.span.end;
            }
            actual.push_str(&input[cursor..]);
            assert_eq!(actual, first.output);
            let nofix =
                format_with_config(&input, &FormatOptions { fix: false }, &settings).unwrap();
            assert_eq!(nofix.output, input);
            assert!(nofix.edits.is_empty());
            let nofix = format_with_config(
                &input,
                &FormatOptions::default(),
                &ResolvedConfig {
                    fix: false,
                    ..settings
                },
            )
            .unwrap();
            assert_eq!(nofix.output, input);
            assert!(nofix.edits.is_empty());
        }
    }
}

#[test]
fn existing_reverse_edit_priorities_remain_bounded_in_warning_only_cli_modes() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    for (input, expected, rules) in [
        (
            "- # H\n",
            "- # H\n",
            json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD023":{"severity":"warning"}}),
        ),
        (
            "- - a\n",
            "-   -   a\n",
            json!({"default":false,"MD007":{"severity":"warning"},"MD030":{"ul_single":3,"severity":"warning"}}),
        ),
        (
            "> - a\n",
            "> - a\n",
            json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD027":{"severity":"warning"}}),
        ),
    ] {
        fs::write(&path, input).unwrap();
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"config":rules}).to_string(),
        )
        .unwrap();
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
                    assert_eq!(status.code(), Some(0));
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("real rule priority did not terminate");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), expected);
            assert!(!fs::read_to_string(&errors).unwrap().contains("Error"));
        }
    }
}
