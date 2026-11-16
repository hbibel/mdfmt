use mdfmt::{
    FormatOptions, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD051":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn fragment_diagnostics_distinguish_missing_and_wrong_case_targets() {
    let input = "# Hello\n\n[a](#hello) [b](#Hello) [c](#missing)\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 2);
    assert_eq!(result.diagnostics[0].span, Some(21..32));
    assert_eq!(result.diagnostics[1].span, Some(33..46));
    assert!(
        result.diagnostics[0]
            .message
            .contains("[Expected: #hello; Actual: #Hello]")
    );
    assert!(!result.diagnostics[1].message.contains("Expected:"));
    let ignored = format_with_config(
        input,
        &FormatOptions { fix: false },
        &config(json!({"ignore_case":true})),
    )
    .unwrap();
    assert_eq!(ignored.diagnostics.len(), 1);
    assert_eq!(ignored.diagnostics[0].span, Some(33..46));
}

#[test]
fn heading_projection_and_duplicate_counters_match_pinned_targets() {
    let input = "# Hello\n# Hello\n# Hello-1\n\n[x](#hello-1) [y](#hello-2)\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    let span = result.diagnostics[0].span.clone().unwrap();
    assert_eq!(&input[span], "[y](#hello-2)");
    for input in [
        "# A &amp; B\n\n[x](#a--b)\n",
        "# Image ![alt](img) `C++` $math$ \\*star\\*\n\n[x](#image--c-math-star)\n",
        "# A {#custom}\n\n[x](#custom) [y](#a-custom)\n",
        "# Café\n\n[x](#caf%C3%A9) [y](#café)\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(
            result.diagnostics.is_empty(),
            "{input:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn definition_and_nested_image_links_keep_source_ownership() {
    for (input, line, literal) in [
        ("[x][id]\n\n[id]: #missing\n", 3, "[id]: #missing"),
        (
            "![alt](#missing) ![[x](#missing)](img)\n",
            1,
            "[x](#missing)",
        ),
        ("[a](\n #missing\n)\n", 1, ""),
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), 1);
        let span = result.diagnostics[0].span.clone().unwrap();
        assert_eq!(&input[span.clone()], literal);
        assert_eq!(
            Source::new(input.into()).position(span.start).unwrap().line,
            line
        );
    }
}

#[test]
fn custom_ignored_patterns_leave_matching_destinations_untouched() {
    let config = config(json!({"ignored_pattern":"^generated-"}));
    let input = "[ignored](#generated-anchor) [unresolved](#other)\n";
    let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert!(result.diagnostics[0].message.contains("unresolved"));
}

#[test]
fn fragment_projections_ranges_and_flags_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md051-observations.json"
    ))
    .unwrap();
    let mut errors = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let settings = config(row["options"].clone());
        let result = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD051");let span=d.span.clone().unwrap();let position=source.position(span.start).unwrap();let text=d.message.strip_prefix("Link fragments should be valid").unwrap();let (detail,context)=if let Some(rest)=text.strip_prefix(" [Expected: "){let (detail,rest)=rest.split_once(']').unwrap();(Some(format!("Expected: {detail}")),rest.strip_prefix(" [Context: \"").and_then(|rest|rest.strip_suffix("\"]")))}else{(None,text.strip_prefix(" [Context: \"").and_then(|rest|rest.strip_suffix("\"]")))};json!({"line":position.line,"range":if span.is_empty(){Value::Null}else{json!([position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),mdfmt::utf16_len(&input[span])])},"detail":detail,"context":context,"severity":if d.severity==mdfmt::Severity::Warning{"warning"}else{"error"}})}).collect();
        if actual != *row["findings"].as_array().unwrap() {
            errors.push(format!(
                "{} {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
        for fix in [false, true] {
            let again = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
            if !fix {
                assert_eq!(again.output, input);
                assert_eq!(again.diagnostics, result.diagnostics);
                assert!(again.edits.is_empty());
            }
            for edit in &again.edits {
                let raw = &input[edit.span.clone()];
                assert_eq!(edit.origin, "MD051");
                assert!(raw.starts_with('#'));
                assert!(raw.eq_ignore_ascii_case(&edit.replacement));
                assert!(
                    raw[1..]
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                );
            }
            let settled =
                format_with_config(&again.output, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(settled.output, again.output);
            assert_eq!(settled.diagnostics, again.diagnostics);
            assert!(settled.edits.is_empty());
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn footnote_calls_contribute_heading_label_data_but_definitions_are_not_destinations() {
    let input = "# Foot[^a]\n\n[^a]: text\n\n[x](#foot) [y](#foota)\n";
    let result =
        format_with_config(input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        &input[result.diagnostics[0].span.clone().unwrap()],
        "[x](#foot)"
    );
}

#[test]
fn fragment_alias_warnings_and_missing_targets_preserve_files_in_every_mode() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let input = "\u{feff}# Heading\r\n\r\n[x](#missing)\r\n";
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"link-fragments":{"severity":severity}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                for _ in 0..2 {
                    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                        .current_dir(dir.path())
                        .args([mode, "doc.md"])
                        .output()
                        .unwrap();
                    assert_eq!(result.status.code(), Some(i32::from(severity == "error")));
                    assert_eq!(
                        String::from_utf8_lossy(&result.stderr)
                            .matches("MD051")
                            .count(),
                        1
                    );
                    assert_eq!(fs::read_to_string(&path).unwrap(), input);
                }
            }
        }
    }
}

#[test]
fn dense_target_lookup_and_repeated_recovered_labels_keep_all_findings() {
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
        json!({"fix":false,"config":{"default":false,"MD051":{"severity":"warning"}}}).to_string(),
    )
    .unwrap();
    let dense = format!(
        "{}{}{}",
        (0..500)
            .map(|index| format!("# Heading {index}\n\n# [Tab{index}](#tab/{index})\n\n"))
            .collect::<String>(),
        (0..1000)
            .map(|index| format!("[link](#Heading-{})\n\n", index % 500))
            .collect::<String>(),
        "z".repeat(65536)
    );
    let images = format!(
        "{}[id]: /url\n{}",
        "![<span id=\"A\">[x](#missing) [ref][id]</span>](img)\n\n".repeat(32),
        (0..128)
            .map(|index| format!("[unrelated{index}]: /url \"{}\"\n", "a".repeat(1024)))
            .collect::<String>()
    );
    for (input, count) in [(dense, 1000), (images, 32)] {
        fs::write(&path, &input).unwrap();
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
                panic!("fragment analysis exceeded work budget");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        assert_eq!(
            fs::read_to_string(&errors)
                .unwrap()
                .matches("Warning MD051")
                .count(),
            count
        );
    }
}

#[test]
fn unique_ascii_fragment_case_repairs_change_only_owned_destinations() {
    for (input, expected, remaining) in [
        (
            "# Hello\n\n[a](#hello) [b](#Hello) [c](#missing)\n",
            "# Hello\n\n[a](#hello) [b](#hello) [c](#missing)\n",
            1,
        ),
        (
            "<a id=\"Foo\"></a>\n\n[x](#foo \"Title\")\n",
            "<a id=\"Foo\"></a>\n\n[x](#Foo \"Title\")\n",
            0,
        ),
    ] {
        let settings = config(json!(true));
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, expected);
        assert_eq!(result.diagnostics.len(), remaining);
        assert_eq!(result.edits.len(), 1);
        let edit = &result.edits[0];
        assert_eq!(edit.origin, "MD051");
        assert!(input[edit.span.clone()].starts_with('#'));
        assert!(input[edit.span.clone()].eq_ignore_ascii_case(&edit.replacement));
        let again = format_with_config(expected, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, expected);
        assert_eq!(again.diagnostics, result.diagnostics);
        assert!(again.edits.is_empty());
        let disabled = ResolvedConfig {
            fix: false,
            ..settings.clone()
        };
        let global_nofix = format_with_config(input, &FormatOptions::default(), &disabled).unwrap();
        assert_eq!(global_nofix.output, input);
        assert!(global_nofix.edits.is_empty());
        let nofix = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
        assert_eq!(nofix.output, input);
        assert!(nofix.edits.is_empty());
        let ignored = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"ignore_case":true})),
        )
        .unwrap();
        assert_eq!(ignored.output, input);
        assert!(ignored.edits.is_empty());
        assert_eq!(ignored.diagnostics.len(), remaining);
    }
}

#[test]
fn ambiguous_or_protected_fragment_targets_are_never_guessed() {
    for input in [
        "<a id=\"Foo\"></a><a id=\"FOO\"></a>\n\n[x](#foo)\n",
        "# Hello\n\n[x][id]\n\n[id]: #Hello\n",
        "# Hello\n\n[x](\n #Hello\n)\n",
        "# Hello\n\n[x](#he%6Clo) [y](#he&#108;lo) [z](#Hell\\o)\n",
        "# Café\n\n[x](#CAFÉ)\n",
        "# Hello\n\n| a |\n| - |\n| [x](#Hello) |\n",
        "# Hello\n\n[x](file.md#Hello) [y](https://example.com/#Hello)\n",
        "# Hello\n\n![[x](#Hello)](img)\n",
    ] {
        let settings = config(json!(true));
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let again =
            format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, input);
        assert_eq!(again.diagnostics, result.diagnostics);
    }
}

#[test]
fn fragment_corrections_follow_heading_reflow_and_directive_permissions() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD051":true,"MD003":{"style":"setext"},"MD013":{"line_length":32},"MD026":true})).unwrap(),..Default::default()};
    let input = "# Heading!\n\nA paragraph with enough words to wrap before this [go](#Heading).\n";
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(
        first.output.starts_with("Heading\n===\n"),
        "{}",
        first.output
    );
    assert!(first.output.contains("[go](#heading)"));
    assert!(!first.output.contains("#Heading"));
    assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert!(again.edits.is_empty());
    let mut reconstructed = String::new();
    let mut cursor = 0;
    for edit in first.edits {
        reconstructed.push_str(&input[cursor..edit.span.start]);
        reconstructed.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    reconstructed.push_str(&input[cursor..]);
    assert_eq!(reconstructed, first.output);
    let input = "<!-- markdownlint-disable-next-line MD051 -->\n# Hello\n\n[a](#Hello) [b](#Hello) <!-- markdownlint-disable-line MD051 -->\n\n[c](#Hello)\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert!(result.output.contains("[a](#Hello) [b](#Hello)"));
    assert!(result.output.ends_with("[c](#hello)\n"));
    assert!(result.diagnostics.is_empty());
}

#[test]
fn case_correction_cli_obeys_alias_severity_and_both_fix_controls() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let input = "\u{feff}# Hello\r\n\r\n[x](#Hello \"Title\")\r\n";
    let expected = input.replace("(#Hello ", "(#hello ");
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(&path, input).unwrap();
            fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"link-fragments":{"severity":severity}}}).to_string()).unwrap();
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
                    Some(i32::from(!fix && severity == "error"))
                );
                assert_eq!(
                    fs::read_to_string(&path).unwrap(),
                    if fix { &expected } else { input }
                );
                assert_eq!(
                    String::from_utf8_lossy(&write.stderr)
                        .matches("MD051")
                        .count(),
                    usize::from(!fix)
                );
            }
        }
    }
}

#[test]
fn dense_unique_corrections_and_ambiguous_targets_remain_bounded_and_idempotent() {
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
        json!({"config":{"default":false,"MD051":{"severity":"warning"}}}).to_string(),
    )
    .unwrap();
    for (input, expected, count) in [
        (
            format!("# Hello\n\n{}", "[x](#Hello)\n".repeat(1000)),
            format!("# Hello\n\n{}", "[x](#hello)\n".repeat(1000)),
            0,
        ),
        (
            format!(
                "<a id=\"Foo\"></a><a id=\"FOO\"></a>\n\n{}",
                "[x](#foo)\n".repeat(1000)
            ),
            format!(
                "<a id=\"Foo\"></a><a id=\"FOO\"></a>\n\n{}",
                "[x](#foo)\n".repeat(1000)
            ),
            1000,
        ),
    ] {
        fs::write(&path, &input).unwrap();
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
                    panic!("fragment corrections exceeded work budget");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), expected);
            assert_eq!(
                fs::read_to_string(&errors)
                    .unwrap()
                    .matches("Warning MD051")
                    .count(),
                count
            );
        }
    }
}
