use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD044":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn proper_names_report_without_rewriting_prose_or_literal_code() {
    for input in ["github\n", "`github`\n", "```text\ngithub\n```\n"] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"names":["GitHub"]})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].message,
            "Proper names should have the correct capitalization [Expected: GitHub; Actual: github]"
        );
        assert_eq!(
            &input[result.diagnostics[0].span.clone().unwrap()],
            "github"
        );
    }
}

#[test]
fn urls_do_not_hide_following_names_in_enabled_code() {
    let input = "`https://example.com github`\n";
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"names":["GitHub"]})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(21..27));
    let disabled = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"names":["GitHub"],"code_blocks":false})),
    )
    .unwrap();
    assert!(disabled.diagnostics.is_empty());
}

#[test]
fn code_url_exclusions_use_logical_content_but_original_source_ranges() {
    for (input, start) in [
        ("    https://github.com github\n", 23),
        (">     https://github.com github\n", 25),
        ("- x\n\n      https://github.com github\n", 30),
    ] {
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(json!({"names":["GitHub"]})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert_eq!(
            result.diagnostics.len(),
            1,
            "{input:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics[0].span, Some(start..start + 6));
    }
}

#[test]
fn bom_does_not_change_list_code_ownership_for_url_or_name_flags() {
    let input = "\u{feff}- item\r\r      https://example.com github\r";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD034":true,"MD044":{"names":["GitHub"],"code_blocks":false}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(result.output, input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn container_continuation_prefixes_are_not_proper_name_data() {
    for input in [
        "> first\n> github\n",
        "- first\n  github\n",
        "> first\n>    github\n",
    ] {
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(json!({"names":["> GitHub"," GitHub"]})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(
            result.diagnostics.is_empty(),
            "{input:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn names_flags_boundaries_priority_and_ranges_match_ordinary_and_policy_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md044-observations.json"
    ))
    .unwrap();
    let mut mismatches = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let settings = config(row["options"].clone());
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let source = Source::new(input.into());
        let mut actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD044");let span=d.span.clone().unwrap();let position=source.position(span.start).unwrap();json!({"line":position.line,"range":[position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),mdfmt::utf16_len(&input[span])],"context":null,"detail":d.message.strip_prefix("Proper names should have the correct capitalization [").unwrap().strip_suffix(']').unwrap(),"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let mut expected = row["findings"].as_array().unwrap().clone();
        let sort = |a: &Value, b: &Value| {
            a["line"]
                .as_u64()
                .cmp(&b["line"].as_u64())
                .then(a["range"][0].as_u64().cmp(&b["range"][0].as_u64()))
                .then(a["detail"].as_str().cmp(&b["detail"].as_str()))
        };
        actual.sort_by(sort);
        expected.sort_by(sort);
        if actual != expected {
            mismatches.push(format!(
                "{}: {input:?}: {actual:?} != {expected:?}",
                row["id"]
            ));
        }
        for fix in [false, true] {
            let again = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(again.output, input);
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn empty_names_terminate_without_hiding_nonempty_names() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("doc.md"), "github\n").unwrap();
    for names in [json!([""]), json!(["", "GitHub"])] {
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"config":{"default":false,"proper-names":{"names":names}}}).to_string(),
        )
        .unwrap();
        let errors = dir.path().join("errors.log");
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
                assert_eq!(
                    status.code(),
                    Some(i32::from(names.as_array().unwrap().len() > 1))
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("empty configured names did not terminate");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            fs::read_to_string(errors).unwrap().matches("MD044").count(),
            usize::from(names.as_array().unwrap().len() > 1)
        );
    }
}

#[test]
fn cli_and_reflow_keep_proper_names_literal_and_warning_only() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"proper-names":{"names":["GitHub"],"severity":"warning"},"MD013":{"line_length":20,"code_blocks":false},"MD026":true})).unwrap(),..Default::default()};
    let input = "# Heading!\n\ngithub in a paragraph that needs wrapping\n\n```text\ngithub\n```\n";
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(result.output.starts_with("# Heading\n"));
    assert!(result.output.ends_with("```text\ngithub\n```\n"));
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.code == "MD044" && d.severity == Severity::Warning)
    );
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn dense_names_and_repeated_images_do_not_repeat_definition_payload_work() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let input = format!(
        "> {}{}\n\n{}{}",
        "github ".repeat(1000),
        "a".repeat(262144),
        "![github](img)\n\n".repeat(100),
        (0..100)
            .map(|index| format!("[d{index}]: /url \"{}\"\n", "a".repeat(1024)))
            .collect::<String>()
    );
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("findings.log");
    fs::write(&path, &input).unwrap();
    fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"fix":false,"config":{"default":false,"proper-names":{"names":["GitHub","Git","C++","Node.js","Rust","ECMAScript","OpenAI"],"severity":"warning"}}}).to_string()).unwrap();
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
            panic!("proper-name scanning exceeded work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), input);
    assert_eq!(
        fs::read_to_string(errors)
            .unwrap()
            .matches("Warning MD044")
            .count(),
        1100
    );
}

#[test]
fn cli_name_aliases_keep_code_and_labels_exact_in_write_and_check_modes() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let input = "\u{feff}![github](img)\r\n\r\n`github`\r\n";
    let path = dir.path().join("doc.md");
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(dir.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"proper-names":{"names":["GitHub"],"severity":severity}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(dir.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(result.status.code(), Some(i32::from(severity == "error")));
                assert_eq!(
                    String::from_utf8_lossy(&result.stderr)
                        .matches("MD044")
                        .count(),
                    2
                );
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}
