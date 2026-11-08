use mdfmt::{
    FormatOptions, FormatResult, Severity, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(options: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD050":options})).unwrap(),
        ..Default::default()
    }
}
fn stable(input: &str, expected: &str, settings: &ResolvedConfig) -> FormatResult {
    let first = format_with_config(input, &FormatOptions::default(), settings).unwrap();
    assert_eq!(first.output, expected, "{input:?}");
    let mut applied = input.to_owned();
    for edit in first.edits.iter().rev() {
        applied.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(applied, first.output);
    let second = format_with_config(&first.output, &FormatOptions::default(), settings).unwrap();
    assert_eq!(second.output, first.output);
    assert_eq!(second.diagnostics, first.diagnostics);
    assert!(second.edits.is_empty());
    first
}

#[test]
fn strong_pairs_change_only_markers_and_settle_once() {
    let input = "__one__ and **two**\n";
    let settings = config(json!({"style":"asterisk"}));
    let raw = format_with_config(input, &FormatOptions { fix: false }, &settings).unwrap();
    assert_eq!(raw.output, input);
    assert_eq!(raw.diagnostics.len(), 2);
    assert_eq!(raw.diagnostics[0].span, Some(0..2));
    assert_eq!(raw.diagnostics[1].span, Some(5..7));
    assert!(raw.diagnostics.iter().all(|d| d.code == "MD050"
        && d.message == "Strong style [Expected: asterisk; Actual: underscore]"));
    let first = stable(input, "**one** and **two**\n", &settings);
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.edits.len(), 2);
    assert!(
        first
            .edits
            .iter()
            .all(|e| e.origin == "MD050" && e.replacement == "**")
    );
}

#[test]
fn strong_diagnostics_match_pinned_ranges_styles_and_permissions() {
    let rows: Value = serde_json::from_str(include_str!(
        "../tools/parser-oracle/md050-observations.json"
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
        let actual:Vec<_>=result.diagnostics.iter().map(|d|{assert_eq!(d.code,"MD050");let span=d.span.clone().unwrap();let a=source.position(span.start).unwrap();let b=source.position(span.end).unwrap();assert_eq!(a.line,b.line);json!({"line":a.line,"column":a.column_utf16-usize::from(a.line==1&&input.starts_with('\u{feff}')),"length":b.column_utf16-a.column_utf16,"detail":d.message.strip_prefix("Strong style [").unwrap().strip_suffix(']').unwrap(),"severity":if d.severity==Severity::Warning{"warning"}else{"error"}})}).collect();
        if actual != *row["findings"].as_array().unwrap() {
            failures.push(format!(
                "{}: {input:?}: {actual:?} != {}",
                row["id"], row["findings"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn emphasis_and_strong_have_independent_styles_on_shared_runs() {
    for (input, emphasis, strong, expected) in [
        ("***both***\n", "underscore", "asterisk", "_**both**_\n"),
        ("***both***\n", "asterisk", "underscore", "*__both__*\n"),
        ("***both***\n", "underscore", "underscore", "___both___\n"),
        ("___both___\n", "asterisk", "asterisk", "***both***\n"),
        (
            "_one_ **two** *three* __four__\n",
            "consistent",
            "consistent",
            "_one_ **two** _three_ **four**\n",
        ),
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(
                json!({"default":false,"MD049":{"style":emphasis},"MD050":{"style":strong}}),
            )
            .unwrap(),
            ..Default::default()
        };
        let result = stable(input, expected, &settings);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn inner_marker_changes_do_not_break_an_astral_adjacent_outer_run() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD049":{"style":"underscore"},"MD050":{"style":"underscore"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let input = "😀***both***😀\n";
    let result = stable(input, input, &settings);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 4);
}

#[test]
fn adjacent_rule_dependencies_settle_in_one_invocation() {
    for (input, emphasis, strong, expected) in [
        ("_one_**two**\n", "asterisk", "underscore", "*one*__two__\n"),
        ("__one__*two*\n", "underscore", "asterisk", "**one**_two_\n"),
        (
            "_one_**two**_three_**four**\n",
            "asterisk",
            "underscore",
            "*one*__two__*three*__four__\n",
        ),
    ] {
        let settings = ResolvedConfig {
            rules: RuleConfig::from_value(
                json!({"default":false,"MD049":{"style":emphasis},"MD050":{"style":strong}}),
            )
            .unwrap(),
            ..Default::default()
        };
        let result = stable(input, expected, &settings);
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn strong_pair_permissions_and_protected_source_are_not_bypassed() {
    for (input, count) in [
        ("[a __b__](url)\n", 2),
        ("![a __b__](img)\n", 2),
        ("| __one__ |\n| --- |\n", 2),
        ("`__one__`\n", 0),
        (
            "__first <!-- markdownlint-disable-next-line MD050 -->\nsecond__\n",
            1,
        ),
    ] {
        let result = stable(input, input, &config(json!({"style":"asterisk"})));
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), count, "{input:?}");
    }
    for (input, style, count) in [
        ("a**one**b\n", "underscore", 0),
        ("😀__one__😀\n", "asterisk", 0),
        ("😀**one**😀\n", "underscore", 2),
        ("é**one**é\n", "underscore", 2),
    ] {
        let result = stable(input, input, &config(json!({"style":style})));
        assert!(result.edits.is_empty());
        assert_eq!(result.diagnostics.len(), count);
    }
}

#[test]
fn marker_styles_compose_with_heading_suffixes_reflow_setext_and_separators() {
    let settings=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD049":{"style":"asterisk"},"MD050":{"style":"asterisk"},"MD003":{"style":"setext"},"MD013":{"line_length":20},"MD026":true,"MD022":true,"MD031":true,"MD032":true})).unwrap(),..Default::default()};
    let result = stable(
        "# _Heading_!\nalpha __beta__ gamma delta epsilon zeta\n- item\n# __End__!\n",
        "*Heading*\n===\n\nalpha **beta** gamma\ndelta epsilon zeta\n\n- item\n\n**End**\n===\n",
        &settings,
    );
    assert!(result.diagnostics.is_empty());
    for origin in ["MD049", "MD050", "MD026", "MD013", "MD003"] {
        assert!(
            result.edits.iter().any(|e| e.origin.contains(origin)),
            "{origin}"
        );
    }
}

#[test]
fn cli_aliases_check_write_and_warning_only_modes_share_marker_permissions() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doc.md");
    let input = "_one_ __two__\n";
    fs::write(&path, input).unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"config":{"default":false,"emphasis-style":{"style":"asterisk"},"strong-style":{"style":"asterisk"}}}"#).unwrap();
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
    assert_eq!(fs::read_to_string(&path).unwrap(), "*one* **two**\n");
    assert_eq!(run("--check").status.code(), Some(0));
    fs::write(&path, input).unwrap();
    fs::write(directory.path().join(".markdownlint-cli2.jsonc"),r#"{"fix":false,"config":{"default":false,"emphasis-style":{"style":"asterisk","severity":"warning"},"strong-style":{"style":"asterisk","severity":"warning"}}}"#).unwrap();
    for mode in ["--check", "--write"] {
        let output = run(mode);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        let errors = String::from_utf8_lossy(&output.stderr);
        assert_eq!(errors.matches("Warning MD049").count(), 2);
        assert_eq!(errors.matches("Warning MD050").count(), 2);
        assert_eq!(fs::read_to_string(&path).unwrap(), input);
    }
}
