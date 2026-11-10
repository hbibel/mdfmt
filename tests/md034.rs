use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD034":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn bare_urls_report_without_wrapping_or_changing_destinations() {
    let input = "go https://example.com now\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(3..22));
    assert_eq!(
        result.diagnostics[0].message,
        "Bare URL used [Context: \"https://example.com\"]"
    );
    for input in [
        "![https://example.com](img)\n",
        "[https://example.com](url)\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn html_sibling_ownership_and_literal_scopes_match_recorded_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md034-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD034");let span=d.span.clone().unwrap();let position=source.position(span.start).unwrap();json!({"line":position.line,"range":[position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),mdfmt::utf16_len(&input[span])],"context":d.message.strip_prefix("Bare URL used [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"detail":null,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        if actual != *row["findings"].as_array().unwrap() {
            mismatches.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
        for fix in [false, true] {
            let again =
                format_with_config(&result.output, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(again.output, input);
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn cli_bare_url_aliases_preserve_tables_and_warning_exit_status() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let input = "| a | b |\r\n| - | - |\r\n| https://example.com | c |\r\n";
    let path = dir.path().join("doc.md");
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(
                dir.path().join(".markdownlint-cli2.jsonc"),
                json!({"fix":fix,"config":{"default":false,"no-bare-urls":{"severity":severity}}})
                    .to_string(),
            )
            .unwrap();
            for mode in ["--check", "--write"] {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(dir.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(result.status.code(), Some(i32::from(severity == "error")));
                assert!(String::from_utf8_lossy(&result.stderr).contains("MD034"));
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}

#[test]
fn dense_bare_urls_and_image_labels_do_not_reparse_unrelated_payloads() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let input = format!(
        "> {}{}\n\n{}[unused]: /url \"{}\"\n",
        "https://example.com ".repeat(1000),
        "a".repeat(262144),
        "![https://example.com](img)\n\n".repeat(100),
        "a".repeat(8192)
    );
    let path = dir.path().join("doc.md");
    fs::write(&path, &input).unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"fix":false,"config":{"default":false,"MD034":{"severity":"warning"}}}).to_string(),
    )
    .unwrap();
    let errors = dir.path().join("findings.log");
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
            panic!("bare-URL extraction exceeded work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), input);
    assert_eq!(
        fs::read_to_string(errors)
            .unwrap()
            .matches("Warning MD034")
            .count(),
        1000
    );
}
