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
fn custom_ignored_patterns_are_explicitly_deferred_until_regex_compatibility() {
    let error =
        RuleConfig::from_value(json!({"default":false,"MD051":{"ignored_pattern":"^generated-"}}))
            .unwrap_err();
    assert_eq!(error.code, "unsupported-rule-pattern");
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
            assert_eq!(again.output, input);
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
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
