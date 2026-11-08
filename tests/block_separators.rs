use mdfmt::{
    FormatOptions, FormatResult, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(extra: Value) -> ResolvedConfig {
    let mut rules = json!({"default":false,"MD022":true,"MD031":true,"MD032":true});
    for (key, value) in extra.as_object().unwrap() {
        rules
            .as_object_mut()
            .unwrap()
            .insert(key.clone(), value.clone());
    }
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    }
}

fn stable(input: &str, expected: &str, settings: &ResolvedConfig) -> FormatResult {
    let first = format_with_config(input, &FormatOptions::default(), settings).unwrap();
    assert_eq!(first.output, expected);
    let mut applied = input.to_owned();
    for edit in first.edits.iter().rev() {
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(
        applied, first.output,
        "reported edits must address original bytes"
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), settings).unwrap();
    assert_eq!(again.output, first.output);
    assert_eq!(again.diagnostics, first.diagnostics);
    assert!(again.edits.is_empty());
    first
}

#[test]
fn shared_root_boundaries_are_inserted_only_once() {
    for (input, expected, count) in [
        (
            "before\n- item\n# After\n",
            "before\n\n- item\n\n# After\n",
            2,
        ),
        (
            "# H\n```rust\nx\n```\n- item\n# After\n",
            "# H\n\n```rust\nx\n```\n\n- item\n\n# After\n",
            3,
        ),
        (
            "- item\r\n```rust\r\nx\r\n```\r\n# H\r\n",
            "- item\r\n\r\n```rust\r\nx\r\n```\r\n\r\n# H\r\n",
            2,
        ),
    ] {
        let first = stable(input, expected, &config(json!({})));
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.edits.len(), count);
    }
}

#[test]
fn compatible_outer_and_inner_prefixes_do_not_block_safe_repairs() {
    for (input, expected) in [
        ("> - item\n# H\n", "> - item\n\n# H\n"),
        (
            "# H\n> ```rust\n> x\n> ```\n",
            "# H\n\n> ```rust\n> x\n> ```\n",
        ),
        ("- item\n> # H\n", "- item\n\n> # H\n"),
    ] {
        let first = stable(input, expected, &config(json!({})));
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.edits.len(), 1);
    }
}

#[test]
fn prefix_choice_satisfies_the_largest_admissible_requirement_in_one_call() {
    let first = stable(
        "- item\n> # H\n",
        "- item\n>\n>\n>\n> # H\n",
        &config(json!({"MD022":{"lines_above":3,"lines_below":-1},"MD012":{"maximum":1}})),
    );
    assert!(first.diagnostics.is_empty());
}

#[test]
fn plain_blank_conflicts_leave_only_stable_unresolved_requirements() {
    let settings = config(json!({"MD022":{"lines_above":2,"lines_below":2},"MD012":{"maximum":1}}));
    let first = stable(
        "before\n- item\n# After\nafter\n",
        "before\n\n- item\n\n# After\n\nafter\n",
        &settings,
    );
    assert_eq!(first.diagnostics.len(), 2);
    assert!(
        first
            .diagnostics
            .iter()
            .all(|d| d.code == "MD022" && d.message.contains("Expected: 2; Actual: 1"))
    );
    let error = RuleConfig::from_value(json!({"default":false,"MD012":{"maximum":0}})).unwrap_err();
    assert_eq!(error.property, "MD012.maximum");
}

#[test]
fn existing_shared_runs_respect_md012_without_second_call_cleanup() {
    let first = stable(
        "# First\n\n## Second\n",
        "# First\n\n\n## Second\n",
        &config(json!({"MD022":{"lines_above":2,"lines_below":2},"MD012":{"maximum":2}})),
    );
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.edits.len(), 1);
    assert_eq!(first.edits[0].span, 8..8);
}

#[test]
fn separators_expose_safe_setext_conversion_before_table_neighbors() {
    let first = stable(
        "before\n# Title\nafter\n",
        "before\n\nTitle\n===\n\nafter\n",
        &config(json!({"MD003":{"style":"setext"}})),
    );
    assert!(first.diagnostics.is_empty());
    let first = stable(
        "# Before\n| h |\n| --- |\n| v |\n# After\n",
        "Before\n===\n\n| h |\n| --- |\n| v |\n\nAfter\n===\n",
        &config(json!({"MD003":{"style":"setext"},"MD058":true})),
    );
    assert!(first.diagnostics.is_empty());
    assert!(first.output.contains("| h |\n| --- |\n| v |\n"));
}

#[test]
fn punctuation_reflow_and_separators_compose_original_byte_edits() {
    let first = stable(
        "# Heading!\nalpha beta gamma delta epsilon zeta\n- item\n# End!\n",
        "# Heading\n\nalpha beta gamma\ndelta epsilon zeta\n\n- item\n\n# End\n",
        &config(json!({"MD026":true,"MD013":{"line_length":20}})),
    );
    assert!(first.diagnostics.is_empty());
    assert!(first.edits.iter().any(|e| e.origin.contains("MD026")));
    assert!(first.edits.iter().any(|e| e.origin.contains("MD013")));
}

#[test]
fn canonical_shared_boundaries_do_not_retarget_an_active_blank_line() {
    let first = stable(
        "# A <!-- markdownlint-disable-next-line MD026 -->\n\n# B!\n",
        "# A <!-- markdownlint-disable-next-line MD026 -->\n\n# B\n",
        &config(
            json!({"MD022":{"lines_above":2,"lines_below":2},"MD012":{"maximum":2},"MD026":true}),
        ),
    );
    assert_eq!(first.diagnostics.len(), 2);
    assert!(first.diagnostics.iter().all(|d| d.code == "MD022"));
    assert!(first.edits.iter().all(|e| e.origin == "MD026"));
}

#[test]
fn unsafe_inner_fence_findings_remain_after_safe_outer_list_repairs() {
    let first = stable(
        "before\n- item\n  ```rust\n  x\n  ```\n  tail\n# After\n",
        "before\n\n- item\n  ```rust\n  x\n  ```\n  tail\n\n# After\n",
        &config(json!({"MD031":{"severity":"warning"}})),
    );
    assert_eq!(first.diagnostics.len(), 2);
    assert!(first.diagnostics.iter().all(|d| d.code == "MD031"));
    let source = Source::new(first.output.clone());
    assert_eq!(
        first
            .diagnostics
            .iter()
            .map(|d| source
                .position(d.span.as_ref().unwrap().start)
                .unwrap()
                .line)
            .collect::<Vec<_>>(),
        [4, 6]
    );
}

#[test]
fn cli_check_write_and_warning_only_modes_share_separator_permissions() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "# H\n- item\n```rust\nx\n```\n";
    let expected = "# H\n\n- item\n\n```rust\nx\n```\n";
    fs::write(&path, input).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD022":true,"MD031":true,"MD032":true}}"#,
    )
    .unwrap();
    let run = |mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap()
    };
    assert_eq!(run("--check").status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    assert_eq!(run("--write").status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(run("--check").status.code(), Some(0));
    fs::write(&path, input).unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"fix":false,"config":{"default":false,"MD022":{"severity":"warning"},"MD031":{"severity":"warning"},"MD032":{"severity":"warning"}}}"#).unwrap();
    for mode in ["--check", "--write"] {
        let output = run(mode);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        let errors = String::from_utf8_lossy(&output.stderr);
        assert_eq!(errors.matches("Warning MD022").count(), 1);
        assert_eq!(errors.matches("Warning MD031").count(), 1);
        assert_eq!(errors.matches("Warning MD032").count(), 2);
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}
