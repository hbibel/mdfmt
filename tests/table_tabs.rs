use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

#[test]
fn tabbed_list_ownership_does_not_invent_tables_for_any_table_rule() {
    for (rule, options) in [
        ("MD055", json!({"style":"leading_only"})),
        ("MD056", json!(true)),
        ("MD058", json!(true)),
        ("MD060", json!({"style":"aligned"})),
        ("MD060", json!({"style":"tight"})),
    ] {
        let input = "-\t| a |\n  | --- |\n  | b | c |\n# After\n";
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(json!({"default":false,rule:options})).unwrap(),
            ..Default::default()
        };
        for fix in [false, true] {
            let result = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(result.output, input, "{rule}");
            assert!(result.edits.is_empty(), "{rule}");
            assert!(
                result.diagnostics.is_empty(),
                "{rule}: {:?}",
                result.diagnostics
            );
        }
    }
}

#[test]
fn cli_does_not_fail_checks_for_tab_indented_non_tables() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "-\t| a |\n  | --- |\n  | b | c |\n# After\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"config":{"default":false,"MD055":{"style":"leading_only"},"MD056":true,"MD058":true,"MD060":{"style":"aligned"}}}"#).unwrap();
    for mode in ["--check", "--write"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args([mode, "doc.md"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}
