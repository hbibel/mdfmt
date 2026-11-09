use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD052":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn missing_references_report_without_fabricating_destinations() {
    let input = "[label][missing]\n";
    let settings = config(json!(true));
    for fix in [false, true] {
        let first = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(first.output, input);
        assert!(first.edits.is_empty());
        assert_eq!(first.diagnostics.len(), 1);
        assert_eq!(first.diagnostics[0].code, "MD052");
        assert_eq!(first.diagnostics[0].span, Some(0..16));
        assert_eq!(
            first.diagnostics[0].message,
            "Reference links and images should use a label that is defined [Missing link or image reference definition: \"missing\"] [Context: \"[label][missing]\"]"
        );
        let again = format_with_config(&first.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(again.output, input);
        assert_eq!(again.diagnostics, first.diagnostics);
        assert!(again.edits.is_empty());
    }
    assert!(
        format_with_config("[missing]\n", &FormatOptions::default(), &settings)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert_eq!(
        format_with_config(
            "[missing]\n",
            &FormatOptions::default(),
            &config(json!({"shortcut_syntax":true}))
        )
        .unwrap()
        .diagnostics
        .len(),
        1
    );
    let defined = "[label][id]\n\n<!-- markdownlint-disable MD052 -->\n[id]: /url\n";
    let result = format_with_config(defined, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, defined);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn reference_ranges_keys_contexts_and_permissions_match_parser_forced_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md052-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD052");let span=d.span.clone().unwrap();let start=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(start.line,end.line);let (detail,context)=d.message.strip_prefix("Reference links and images should use a label that is defined [").unwrap().split_once("] [Context: \"").unwrap();json!({"line":start.line,"range":[start.column_utf16-usize::from(start.line==1&&input.starts_with('\u{feff}')),end.column_utf16-start.column_utf16],"context":context.strip_suffix("\"]").unwrap(),"detail":detail,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let expected = row["parserForced"].as_array().unwrap();
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
fn cli_reference_aliases_warn_without_rewriting_missing_targets() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "[label][missing]\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":{"default":false,"reference-links-images":{"severity":"warning"}}})
            .to_string(),
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&result.stderr)
                .matches("Warning MD052")
                .count(),
            1
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}

#[test]
fn dense_missing_references_do_not_reparse_the_document_per_label() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    let input = format!(
        "{}{}\n",
        "[label][missing] ".repeat(1000),
        "a".repeat(262144)
    );
    let path = directory.path().join("doc.md");
    fs::write(&path, &input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":{"default":false,"MD052":true}}).to_string(),
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
            panic!("dense missing labels exceeded reference-analysis work budget");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(child.wait().unwrap().code(), Some(1));
    assert_eq!(fs::read_to_string(path).unwrap(), input);
}
