use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD053":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn unused_and_duplicate_definitions_remain_source_content() {
    let settings = config(json!(true));
    for (input, span, detail, context) in [
        (
            "[unused]: /url\n",
            0..14,
            "Unused link or image reference definition: \"unused\"",
            "[unused]: /url",
        ),
        (
            "[id]: /url\n[id]: /url\n\n[text][id]\n",
            11..21,
            "Duplicate link or image reference definition: \"id\"",
            "[id]: /url",
        ),
    ] {
        for fix in [false, true] {
            let first = format_with_config(input, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(first.output, input);
            assert!(first.edits.is_empty());
            assert_eq!(first.diagnostics.len(), 1);
            assert_eq!(first.diagnostics[0].code, "MD053");
            assert_eq!(first.diagnostics[0].span, Some(span.clone()));
            assert_eq!(
                first.diagnostics[0].message,
                format!(
                    "Link and image reference definitions should be needed [{detail}] [Context: \"{context}\"]"
                )
            );
            let again =
                format_with_config(&first.output, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(again.output, input);
            assert_eq!(again.diagnostics, first.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
}

#[test]
fn definition_ranges_keys_contexts_and_permissions_match_parser_forced_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md053-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD053");let span=d.span.clone().unwrap();let start=source.position(span.start).unwrap();let end=source.position(span.end).unwrap();assert_eq!(start.line,end.line);let (detail,context)=d.message.strip_prefix("Link and image reference definitions should be needed [").unwrap().split_once("] [Context: \"").unwrap();json!({"line":start.line,"range":[start.column_utf16-usize::from(start.line==1&&input.starts_with('\u{feff}')),end.column_utf16-start.column_utf16],"context":context.strip_suffix("\"]").unwrap(),"detail":detail,"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
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
fn reference_findings_follow_reflow_without_deleting_unused_definitions() {
    let input = "# title!\n\nalpha beta [label][missing] gamma delta\n\n[unused]: /url\n";
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD052":true,"MD053":true,"MD013":{"line_length":20},"MD026":true})).unwrap(),..Default::default()};
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(first.output.starts_with("# title\n\n"));
    assert!(first.output.ends_with("\n[unused]: /url\n"));
    assert_eq!(
        first
            .diagnostics
            .iter()
            .filter(|d| d.code == "MD052")
            .count(),
        1
    );
    assert_eq!(
        first
            .diagnostics
            .iter()
            .filter(|d| d.code == "MD053")
            .count(),
        1
    );
    let again = format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, first.output);
    assert_eq!(again.diagnostics, first.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn cli_definition_alias_warnings_never_delete_unused_content() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let input = "[unused]: /url\n";
    let path = directory.path().join("doc.md");
    fs::write(&path, input).unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),json!({"config":{"default":false,"link-image-reference-definitions":{"severity":"warning"}}}).to_string()).unwrap();
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
                .matches("Warning MD053")
                .count(),
            1
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}

#[test]
fn disabled_first_definitions_still_anchor_duplicates_and_disabled_uses_still_count() {
    let input = "<!-- markdownlint-disable-next-line MD053 -->\n[id]: /one\n[id]: /two\n\n<!-- markdownlint-disable-next-line MD053 -->\n[text][id]\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(result.diagnostics.len(), 1);
    assert!(result.diagnostics[0].message.contains("Duplicate"));
    let source = Source::new(input.into());
    assert_eq!(
        source
            .position(result.diagnostics[0].span.as_ref().unwrap().start)
            .unwrap()
            .line,
        3
    );
}
