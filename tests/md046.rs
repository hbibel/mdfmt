use mdfmt::{
    FormatOptions, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD046":options})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn mixed_code_styles_report_without_converting_code() {
    let input = "```rs\nlet x = 1;\n```\n\n    code\n";
    let result =
        format_with_config(input, &FormatOptions::default(), &config(json!(true))).unwrap();
    assert_eq!(result.output, input);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].span, Some(22..22));
    assert_eq!(
        result.diagnostics[0].message,
        "Code block style [Expected: fenced; Actual: indented]"
    );
}

#[test]
fn explicit_styles_and_disabled_first_anchors_preserve_consistency() {
    let input = "<!-- markdownlint-disable MD046 -->\n```\nx\n```\n<!-- markdownlint-enable MD046 -->\n\n    y\n";
    for (options, line) in [
        (json!(true), 7),
        (json!({"style":"fenced"}), 7),
        (json!({"style":"indented"}), 0),
    ] {
        let result =
            format_with_config(input, &FormatOptions { fix: false }, &config(options)).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.diagnostics.len(), usize::from(line > 0));
        if line > 0 {
            assert_eq!(
                Source::new(input.into())
                    .position(result.diagnostics[0].span.as_ref().unwrap().start)
                    .unwrap()
                    .line,
                line
            );
        }
    }
}

#[test]
fn code_style_ownership_ranges_and_permissions_match_recorded_observations() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md046-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD046");let span=d.span.clone().unwrap();assert!(span.is_empty());let position=source.position(span.start).unwrap();assert_eq!(position.column_utf16-usize::from(position.line==1&&input.starts_with('\u{feff}')),1);json!({"line":position.line,"range":null,"context":null,"detail":d.message.strip_prefix("Code block style [").unwrap().strip_suffix(']').unwrap(),"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        if actual != *row["findings"].as_array().unwrap() {
            mismatches.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
        for fix in [false, true] {
            let again =
                format_with_config(&result.output, &FormatOptions { fix }, &settings).unwrap();
            assert_eq!(again.output, result.output);
            assert_eq!(again.diagnostics, result.diagnostics);
            assert!(again.edits.is_empty());
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn style_diagnostics_follow_reflow_and_heading_fixes_without_changing_code() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"code-block-style":{"style":"fenced","severity":"warning"},"MD013":{"line_length":20,"code_blocks":false},"MD026":true})).unwrap(),..Default::default()};
    let input = "# Heading!\n\nwords in a paragraph that needs wrapping\n\n    code with a very long literal value\n";
    let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert!(result.output.starts_with("# Heading\n"));
    assert!(
        result
            .output
            .ends_with("    code with a very long literal value\n")
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "MD046");
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    let again = format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(again.output, result.output);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.edits.is_empty());
}

#[test]
fn blank_lines_only_join_indented_code_with_the_same_container_owner() {
    for (input, lines) in [
        (">     a\n> \n>     b\n", vec![1]),
        (">     a\n\n>     b\n", vec![1, 3]),
        ("-     a\n  \n      b\n", vec![1]),
        ("-     a\n\n-     b\n", vec![1, 3]),
    ] {
        let result = format_with_config(
            input,
            &FormatOptions { fix: false },
            &config(json!({"style":"fenced"})),
        )
        .unwrap();
        assert_eq!(result.output, input);
        let source = Source::new(input.into());
        let actual: Vec<_> = result
            .diagnostics
            .iter()
            .map(|d| {
                source
                    .position(d.span.as_ref().unwrap().start)
                    .unwrap()
                    .line
            })
            .collect();
        assert_eq!(actual, lines, "{input:?}");
    }
}

#[test]
fn cli_code_style_aliases_preserve_bytes_in_check_write_and_no_fix_modes() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "\u{feff}    code\r\n";
    fs::write(&path, input).unwrap();
    for severity in ["error", "warning"] {
        for fix in [false, true] {
            fs::write(directory.path().join(".markdownlint-cli2.jsonc"),json!({"fix":fix,"config":{"default":false,"code-block-style":{"style":"fenced","severity":severity}}}).to_string()).unwrap();
            for mode in ["--check", "--write"] {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(directory.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(result.status.code(), Some(i32::from(severity == "error")));
                assert!(String::from_utf8_lossy(&result.stderr).contains("MD046"));
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}
