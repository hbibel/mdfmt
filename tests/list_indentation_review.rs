use serde_json::{Value, json};
use std::{
    fs,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn wait(mut child: Child, budget: Duration) -> std::process::ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if start.elapsed() > budget {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("list normalization did not terminate within {budget:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn converges(input: &str, expected: &str, rules: Value, code: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    fs::write(&path, input).unwrap();
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        json!({"config":rules}).to_string(),
    )
    .unwrap();
    for mode in ["--write", "--write", "--check"] {
        let child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(dir.path())
            .args([mode, "doc.md"])
            .stdout(Stdio::null())
            .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
            .spawn()
            .unwrap();
        assert_eq!(wait(child, Duration::from_secs(5)).code(), Some(1));
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        let errors = fs::read_to_string(&errors).unwrap();
        assert_eq!(errors.matches("Error MD").count(), 1, "{errors}");
        assert_eq!(
            errors.matches(&format!("Error {code}")).count(),
            1,
            "{errors}"
        );
    }
}

#[test]
fn root_heading_indentation_cannot_cycle_with_md023() {
    converges(
        "- # H\n",
        "- # H\n",
        json!({"default":false,"MD007":{"start_indented":true},"MD023":true}),
        "MD007",
    );
}

#[test]
fn disabled_heading_indentation_anchor_cannot_cycle_with_md005() {
    let input = " - # A <!-- markdownlint-disable-line MD023 -->\n- # B\n";
    converges(
        input,
        input,
        json!({"default":false,"MD005":true,"MD023":true}),
        "MD005",
    );
}

#[test]
fn nested_same_line_indentation_insertion_cannot_cycle_with_md030() {
    converges(
        "- - a\n",
        "- - a\n",
        json!({"default":false,"MD007":{"indent":4},"MD030":true}),
        "MD007",
    );
}

#[test]
fn nested_same_line_indentation_deletion_cannot_cycle_with_md030() {
    converges(
        "- - a\n",
        "-   -   a\n",
        json!({"default":false,"MD007":true,"MD030":{"ul_single":3}}),
        "MD007",
    );
}

#[cfg(target_os = "linux")]
#[test]
fn admitted_intermediate_targets_keep_linear_memory_in_both_fix_modes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    let input = "- a\n".repeat(4000);
    fs::write(&path, &input).unwrap();
    for (options, fix, count) in [
        (json!(false), false, 0),
        (
            json!({"start_indented":true,"start_indent":16000}),
            false,
            4000,
        ),
        (
            json!({"start_indented":true,"start_indent":16000}),
            true,
            4000,
        ),
    ] {
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"fix":fix,"config":{"default":false,"MD007":options}}).to_string(),
        )
        .unwrap();
        let child = Command::new("bash")
            .current_dir(dir.path())
            .args([
                "-c",
                "ulimit -v 131072\nexec \"$1\" --write doc.md",
                "mdfmt-memory-regression",
                env!("CARGO_BIN_EXE_mdfmt"),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
            .spawn()
            .unwrap();
        let status = wait(child, Duration::from_secs(30));
        assert_eq!(
            status.code(),
            Some(i32::from(count > 0)),
            "{status:?}: {}",
            fs::read_to_string(&errors).unwrap()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
        let errors = fs::read_to_string(&errors).unwrap();
        assert_eq!(errors.matches("Error MD007").count(), count);
        if count > 0 {
            assert!(errors.contains("Expected: 16000; Actual: 0"));
        }
    }
}

#[test]
fn disabled_opposing_rules_still_allow_safe_indentation_repairs() {
    use mdfmt::{
        FormatOptions,
        config::{ResolvedConfig, RuleConfig},
        format_with_config,
    };
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
            "- - a\n",
            "-   - a\n",
            json!({"default":false,"MD007":{"indent":4},"MD030":false}),
        ),
        (
            "- - a <!-- markdownlint-disable-line MD030 -->\n",
            "-   - a <!-- markdownlint-disable-line MD030 -->\n",
            json!({"default":false,"MD007":{"indent":4},"MD030":true}),
        ),
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(rules).unwrap(),
            ..Default::default()
        };
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, expected);
        assert!(result.diagnostics.is_empty());
        let again = format_with_config(expected, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, expected);
        assert!(again.edits.is_empty());
    }
}

#[test]
fn aggregate_budget_keeps_safe_dense_start_indentation_and_exact_edit_ranges() {
    use mdfmt::{
        FormatOptions,
        config::{ResolvedConfig, RuleConfig},
        format_with_config,
    };
    let input = "- a\n".repeat(50);
    let expected = "  - a\n".repeat(50);
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD007":{"start_indented":true}}))
            .unwrap(),
        ..Default::default()
    };
    let first = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, expected);
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.edits.len(), 50);
    assert!(
        first
            .edits
            .iter()
            .all(|edit| edit.span.is_empty() && edit.replacement == "  " && edit.origin == "MD007")
    );
    let again = format_with_config(&expected, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, expected);
    assert!(again.edits.is_empty());
}
