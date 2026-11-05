use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD058":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn root_table_separators_are_added_without_changing_table_source() {
    let table = "| a | b |\n| --- | --- |\n| c | d |\n";
    let input = format!("# before\n{table}# after\n");
    let check =
        format_with_config(&input, &FormatOptions { fix: false }, &config(json!(true))).unwrap();
    assert_eq!(check.output, input);
    assert_eq!(check.diagnostics.len(), 2);
    let first =
        format_with_config(&input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, format!("# before\n\n{table}\n# after\n"));
    assert!(first.diagnostics.is_empty());
    let second = format_with_config(
        &first.output,
        &FormatOptions::default(),
        &config(json!(true)),
    )
    .unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}

#[test]
fn physical_neighbors_and_contexts_match_pinned_table_separator_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md058-observations.json"
    ))
    .unwrap();
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
            assert_eq!(finding.code,"MD058");let position=source.position(finding.span.as_ref().unwrap().start).unwrap();
            json!({"line":position.line,"range":null,"context":finding.message.strip_prefix("Tables should be surrounded by blank lines [Context: \"").unwrap().strip_suffix("\"]").unwrap(),"severity":if finding.severity==Severity::Warning{"warning"}else{"error"}})
        }).collect();
        assert_eq!(
            actual,
            *row["findings"].as_array().unwrap(),
            "{}",
            row["id"]
        );
    }
}

#[test]
fn uncertain_container_tables_are_preserved_with_remaining_findings() {
    let input = "- | a | b |\n  | --- | --- |\n  | c | d |\n# after\n";
    let first = format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(first.output, input);
    assert_eq!(first.diagnostics.len(), 1);
}

#[test]
fn crlf_and_disabled_fix_permissions_are_retained() {
    let input = "# before\r\n| a | b |\r\n| --- | --- |\r\n| c | d |\r\n# after\r\n";
    let first = format_with_config(
        input,
        &FormatOptions::default(),
        &config(json!({"severity":"warning"})),
    )
    .unwrap();
    assert_eq!(
        first.output,
        input
            .replace("before\r\n|", "before\r\n\r\n|")
            .replace("d |\r\n#", "d |\r\n\r\n#")
    );
    let mut settings = config(json!({"severity":"warning"}));
    settings.fix = false;
    let check = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(check.output, input);
    assert_eq!(check.diagnostics.len(), 2);
    assert!(
        check
            .diagnostics
            .iter()
            .all(|finding| finding.severity == Severity::Warning)
    );
}

#[test]
fn cli_check_and_write_apply_only_safe_table_separator_insertions() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(
        dir.path().join(".markdownlint-cli2.jsonc"),
        r#"{"config":{"default":false,"MD058":true}}"#,
    )
    .unwrap();
    let input = "# before\n| a | b |\n| --- | --- |\n| c | d |\n# after\n";
    fs::write(&path, input).unwrap();
    let check = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--check", "doc.md"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    let write = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .output()
        .unwrap();
    assert!(write.status.success(), "{:?}", write);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        input
            .replace("before\n|", "before\n\n|")
            .replace("d |\n#", "d |\n\n#")
    );
}

#[test]
fn separator_insertions_do_not_retarget_other_rules_next_line_directives() {
    let input = "| a |\n| --- |\n| c <!-- markdownlint-disable-next-line MD026 --> |\n# Heading!\n";
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD058":true,"MD026":true})).unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, input);
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(first.diagnostics[0].code, "MD058");
    let second = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
    assert_eq!(second.diagnostics[0].code, "MD058");
}

#[test]
fn excluded_front_matter_is_not_a_nonblank_table_neighbor() {
    for metadata in [
        "---\ntitle: x\n---\n",
        "+++\ntitle = 'x'\n+++\n",
        "{\n\"title\": \"x\"\n}\n",
    ] {
        for bom in ["", "\u{feff}"] {
            let input = format!("{bom}{metadata}| a | b |\n| --- | --- |\n| c | d |\n");
            for fix in [false, true] {
                let result =
                    format_with_config(&input, &FormatOptions { fix }, &config(json!(true)))
                        .unwrap();
                assert_eq!(result.output, input);
                assert!(
                    result.diagnostics.is_empty(),
                    "{input:?}: {:?}",
                    result.diagnostics
                );
            }
        }
    }
}
