use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD037":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn bare_emphasis_spaces_are_reported_without_creating_emphasis() {
    let input = "alpha * beta * omega\n";
    let settings = config(json!(true));
    for fix in [false, true] {
        let first = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(first.output, input);
        assert!(first.edits.is_empty());
        assert_eq!(
            first
                .diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            [7..8, 12..13]
        );
        assert_eq!(
            first
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Spaces inside emphasis markers [Context: \"* b\"]",
                "Spaces inside emphasis markers [Context: \"a *\"]"
            ]
        );
        let second = format_with_config(&first.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(second.output, first.output);
        assert_eq!(second.diagnostics, first.diagnostics);
        assert!(second.edits.is_empty());
    }
}

#[test]
fn bare_markers_do_not_pair_across_direct_token_parents_or_protected_units() {
    for (input, count) in [
        ("alpha * beta ** omega\n", 0),
        ("alpha _* beta_ gamma * omega\n", 0),
        ("alpha * ` beta *` omega\n", 0),
        ("alpha * [ beta *](url) omega\n", 0),
        ("alpha * ![ beta *](img) omega\n", 0),
        ("alpha \\* beta * omega\n", 0),
        ("alpha _* beta * gamma_ omega\n", 0),
        ("| alpha * beta * omega |\n| --- |\n", 2),
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), count, "{input:?}");
    }
}

#[test]
fn coalesced_parent_data_still_retains_markers_next_to_non_data_tokens() {
    for input in [
        "> _`a`*\n> *`b`_\n",
        "> [`a`*\n> *`b`](url)\n",
        "> _&amp;*\n> *&amp;_\n",
        "> _\\!*\n> *\\!_\n",
    ] {
        let result =
            format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            result.diagnostics[0].message,
            "Spaces inside emphasis markers [Context: \"> *\"]"
        );
    }
}

#[test]
fn emphasis_space_ranges_contexts_and_permissions_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md037-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD037");let span=d.span.clone().unwrap();let p=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(p.line,end.line);json!({"line":p.line,"range":if span.is_empty(){Value::Null}else{json!([p.column_utf16-usize::from(p.line==1&&input.starts_with('\u{feff}')),end.column_utf16-p.column_utf16])},"context":d.message.strip_prefix("Spaces inside emphasis markers [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"detail":Value::Null,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        let expected: Vec<_> = row["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let mut f = f.clone();
                f.as_object_mut().unwrap().remove("contextUtf16");
                f
            })
            .collect();
        if actual != expected {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {expected:?}",
                row["id"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn cli_alias_warnings_do_not_infer_emphasis_or_rewrite_files() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "alpha * beta * omega\n";
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"no-space-in-emphasis":{"severity":"warning"}}}"#,
    )
    .unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr)
                .matches("Warning MD037")
                .count(),
            2
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}
