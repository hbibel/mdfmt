use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD054":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn disallowed_link_styles_preserve_labels_titles_and_destinations() {
    for (input, options, span) in [
        ("[text][id]\n\n[id]: /url\n", json!({"full":false}), 0..10),
        ("[text][ID]\n\n[id]: /url\n", json!({"full":false}), 0..10),
        (
            "[label](https://example.com \"title\")\n",
            json!({"inline":false}),
            0..36,
        ),
        ("[text]()\n", json!({"inline":false}), 0..8),
    ] {
        for fix in [false, true] {
            let result =
                format_with_config(input, &FormatOptions { fix }, &config(options.clone()))
                    .unwrap();
            assert_eq!(result.output, input);
            assert!(result.edits.is_empty());
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(result.diagnostics[0].code, "MD054");
            assert_eq!(result.diagnostics[0].span, Some(span.clone()));
            let again = format_with_config(input, &FormatOptions { fix }, &config(options.clone()))
                .unwrap();
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
}

#[test]
fn non_ascii_after_backslashes_in_resource_fields_never_splits_utf8() {
    for input in [
        "[label](<u\\😀>)\n",
        "[label](url \"a\\😀\")\n",
        "[label](url 'a\\é')\n",
        "[label](url (a\\界))\n",
    ] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"inline":false})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].span, Some(0..input.len() - 1));
    }
}

#[test]
fn style_ranges_contexts_options_and_policies_match_recorded_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md054-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let source = Source::new(input.into());
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(row["options"].clone()),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD054");let span=d.span.clone().unwrap();let start=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(start.line,end.line);json!({"line":start.line,"range":if span.is_empty(){Value::Null}else{json!([start.column_utf16-usize::from(start.line==1&&input.starts_with('\u{feff}')),end.column_utf16-start.column_utf16])},"context":d.message.strip_prefix("Link and image style [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"detail":Value::Null,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let expected = row["nativePolicy"]
            .as_array()
            .or_else(|| row["parserForced"].as_array())
            .unwrap();
        if actual != *expected {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {expected:?}",
                row["id"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn url_inline_uses_absolute_urls_and_retains_title_and_image_exceptions() {
    let input = "[https://example.com](https://example.com)\n";
    let result = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"url_inline":false})),
    )
    .unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(0..42));
    assert_eq!(
        result.diagnostics[0].message,
        "Link and image style [Context: \"[https://example.com](https://...\"]"
    );
    for input in [
        "[https://example.com](https://example.com \"title\")\n",
        "![https://example.com](https://example.com)\n",
        "[relative/path](relative/path)\n",
    ] {
        let result = format_with_config(
            input,
            &FormatOptions::default(),
            &config(json!({"url_inline":false})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn style_findings_follow_reflow_without_rewriting_inline_links() {
    let input = "alpha beta [label](url) gamma delta\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD054":{"inline":false},"MD013":{"line_length":20}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, "alpha beta\n[label](url) gamma\ndelta\n");
    assert_eq!(result.diagnostics.len(), 1);
    let span = result.diagnostics[0].span.clone().unwrap();
    assert_eq!(&result.output[span], "[label](url)");
    let mut applied = input.to_owned();
    for edit in result.edits.iter().rev() {
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(applied, result.output);
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn dense_style_only_links_do_not_build_unused_reference_records_or_expand_each_line() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    let input = format!("> {}{}\n", "[label](url) ".repeat(1000), "a".repeat(262144));
    let path = directory.path().join("doc.md");
    fs::write(&path, &input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":{"default":false,"MD054":{"inline":false,"severity":"warning"}}})
            .to_string(),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .args(["--check", "doc.md"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("dense style-only links exceeded the extraction work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert_eq!(fs::read_to_string(path).unwrap(), input);
}

#[test]
fn cli_reference_aliases_preserve_source_in_check_write_and_no_fix_modes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "[label][missing]\n\n[unused]: /url\n\n[visible](https://example.com \"title\")\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(directory.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"reference-links-images":{"severity":severity},"link-image-reference-definitions":{"severity":severity},"link-image-style":{"severity":severity,"inline":false}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(directory.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.code(),
                    Some(if severity == "error" { 1 } else { 0 })
                );
                assert!(result.stdout.is_empty());
                let stderr = String::from_utf8_lossy(&result.stderr);
                for rule in ["MD052", "MD053", "MD054"] {
                    assert_eq!(stderr.matches(rule).count(), 1, "{stderr}");
                }
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}
