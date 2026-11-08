use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD022":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn heading_separators_preserve_caption_and_settle_once() {
    let input = "before\n# Heading\nafter\n";
    let settings = config(json!(true));
    let raw = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(raw.output, input);
    assert_eq!(raw.diagnostics.len(), 2);
    assert_eq!(raw.diagnostics[0].span, Some(7..7));
    assert_eq!(
        raw.diagnostics[0].message,
        "Headings should be surrounded by blank lines [Expected: 1; Actual: 0; Above; Context: \"# Heading\"]"
    );
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, "before\n\n# Heading\n\nafter\n");
    assert!(first.diagnostics.is_empty());
    assert!(
        first
            .edits
            .iter()
            .all(|e| e.span.is_empty() && e.origin == "MD022")
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert!(again.edits.is_empty());
}

#[test]
fn heading_neighbors_counts_and_context_match_pinned_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md022-observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
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
        let actual:Vec<_>=result.diagnostics.iter().map(|finding|{
            assert_eq!(finding.code,"MD022");
            let span=finding.span.clone().unwrap();assert!(span.is_empty());
            let position=source.position(span.start).unwrap();
            assert_eq!(position.column_utf16,1+usize::from(position.line==1&&input.starts_with('\u{feff}')));
            json!({"line":position.line,"message":finding.message,"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        let expected:Vec<_>=row["findings"].as_array().unwrap().iter().map(|f|json!({"line":f["line"],"message":format!("Headings should be surrounded by blank lines [{}; Context: \"{}\"]",f["detail"].as_str().unwrap(),f["context"].as_str().unwrap()),"severity":f["severity"]})).collect();
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
fn arrays_negative_sides_and_quote_prefixes_have_stable_repairs() {
    for (input, options, expected) in [
        (
            "before\n# One\n## Two\n### Three\nafter\n",
            json!({"lines_above":[2,0],"lines_below":-1}),
            "before\n\n\n# One\n## Two\n\n### Three\nafter\n",
        ),
        (
            "> before\n> # H\n> after\n",
            json!(true),
            "> before\n>\n> # H\n>\n> after\n",
        ),
        (
            "before\r\n# H\r\nafter\r\n",
            json!(true),
            "before\r\n\r\n# H\r\n\r\nafter\r\n",
        ),
    ] {
        let settings = config(options);
        let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(first.output, expected);
        assert!(first.diagnostics.is_empty());
        let again =
            format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, first.output);
        assert!(again.edits.is_empty());
    }
}

#[test]
fn list_tightness_metadata_and_directive_targets_are_not_changed() {
    for input in [
        "- item\n  # H\n  after\n",
        "---\ntitle: T\n---\n# H\n",
        "# First <!-- markdownlint-disable-next-line MD026 -->\n# Second!\n",
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(json!({"default":false,"MD022":true,"MD026":true}))
                .unwrap(),
            ..Default::default()
        };
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        if !input.starts_with("---") {
            assert_eq!(result.diagnostics.len(), 2);
        }
    }
}

#[test]
fn md012_conflicts_leave_stable_unresolved_heading_requirements() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD022":{"lines_above":2,"lines_below":2},"MD012":{"maximum":1}})).unwrap(),..Default::default()};
    let first =
        format_with_config("before\n# H\nafter\n", &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, "before\n\n# H\n\nafter\n");
    assert_eq!(first.diagnostics.len(), 2);
    assert!(
        first
            .diagnostics
            .iter()
            .all(|f| f.code == "MD022" && f.message.contains("Expected: 2; Actual: 1"))
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert!(again.edits.is_empty());
    assert_eq!(again.diagnostics, first.diagnostics);
}

#[cfg(target_os = "linux")]
#[test]
fn oversized_separator_batches_are_withheld_atomically() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "before\n# One\nmiddle\n## Two\nafter\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD022":{"lines_above":600000,"lines_below":0}}}"#,
    )
    .unwrap();
    let mut command = Command::new("sh");
    command.current_dir(directory.path()).args([
        "-c",
        "ulimit -v 131072; exec \"$1\" --write doc.md",
        "mdfmt-resource-check",
        env!("CARGO_BIN_EXE_mdfmt"),
    ]);
    let output = bounded_cli(command);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .matches("Error MD022")
            .count(),
        2
    );
}

#[test]
fn shared_existing_blank_runs_are_not_double_inserted() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD022":{"lines_above":2,"lines_below":2},"MD012":{"maximum":2}})).unwrap(),..Default::default()};
    let first = format_with_config(
        "# First\n\n## Second\n",
        &FormatOptions::default(),
        &settings,
    )
    .unwrap();
    assert_eq!(first.output, "# First\n\n\n## Second\n");
    assert!(first.diagnostics.is_empty());
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert!(again.edits.is_empty());
}

#[test]
fn default_rule_set_repairs_heading_blanks_in_cli_modes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "# Heading\ntext\n";
    let expected = "# Heading\n\ntext\n";
    fs::write(&path, input).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args(args)
            .output()
            .unwrap()
    };
    let stdout = run(&["doc.md"]);
    assert_eq!(stdout.status.code(), Some(0));
    assert_eq!(stdout.stdout, expected.as_bytes());
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    let check = run(&["--check", "doc.md"]);
    assert_eq!(check.status.code(), Some(1));
    assert!(check.stdout.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    let write = run(&["--write", "doc.md"]);
    assert_eq!(write.status.code(), Some(0));
    assert!(write.stdout.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(run(&["--check", "doc.md"]).status.code(), Some(0));
}

#[test]
fn large_unsigned_counts_are_not_silently_replaced_with_the_default() {
    let settings = config(json!({"lines_above":u64::MAX,"lines_below":0}));
    let input = "before\n# H\nafter\n";
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .message
            .contains("Expected: 18446744073709551615; Actual: 0; Above")
    );
}

fn bounded_cli(mut command: std::process::Command) -> std::process::Output {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(3) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("MD022 did not finish a tiny document with a large blank-line requirement");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn virtual_file_boundary_blanks_do_not_loop_to_the_configured_count() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("doc.md"), "# H\n").unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD022":{"lines_above":1000000000000,"lines_below":0}}}"#,
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mdfmt"));
    command
        .current_dir(directory.path())
        .args(["--check", "doc.md"]);
    let output = bounded_cli(command);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn oversized_blank_insertions_remain_findings_instead_of_exhausting_memory() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "before\n# H\nafter\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD022":{"lines_above":1000000000000,"lines_below":0}}}"#,
    )
    .unwrap();
    let mut command = Command::new("sh");
    command.current_dir(directory.path()).args([
        "-c",
        "ulimit -v 131072; exec \"$1\" --write doc.md",
        "mdfmt-resource-check",
        env!("CARGO_BIN_EXE_mdfmt"),
    ]);
    let output = bounded_cli(command);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error MD022"), "{stderr}");
    assert!(!stderr.contains("allocation"), "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
}
