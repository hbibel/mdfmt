use mdfmt::{
    FormatOptions, ProcessingError,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

const INPUT: &str = "- item\n---\n1. item\n-\n  ```\n    code\n1. item\n";

#[test]
fn native_parser_failures_return_errors_instead_of_unwinding_through_the_library() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false})).unwrap(),
        ..Default::default()
    };
    for input in [INPUT, "- ```\n  x\n1. item\n# After\n"] {
        let result = std::panic::catch_unwind(|| {
            format_with_config(input, &FormatOptions::default(), &settings)
        });
        assert!(
            result.is_ok(),
            "native parser panic crossed the public library boundary"
        );
        assert!(
            matches!(result.unwrap(),Err(ProcessingError::Parse(message)) if message.contains("native parser panicked"))
        );
    }
}

#[test]
fn parser_failure_is_an_operational_cli_error_without_writes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    fs::write(&path, INPUT).unwrap();
    fs::write(
        directory.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false}}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(directory.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Markdown parsing failed: native parser panicked")
    );
    assert_eq!(fs::read_to_string(path).unwrap(), INPUT);
}
