use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn config(rules: Value) -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    }
}

#[test]
fn complete_pipeline_composes_original_ranges_and_settles_across_rule_families() {
    for (input, expected, rules) in [
        (
            "# Heading!\n\nA paragraph with enough words to wrap before this [go](#Heading).\n",
            None,
            json!({"default":false,"MD026":true,"MD003":{"style":"setext"},"MD013":{"line_length":32},"MD051":true}),
        ),
        (
            ">  1.  one\n> 3.   two\n\n- a\n  - b\n",
            Some("> 1. one\n> 2. two\n\n- a\n    - b\n"),
            json!({"default":false,"MD027":true,"MD029":true,"MD030":true,"MD005":true,"MD007":{"indent":4}}),
        ),
        (
            "# Heading!\n*one* **two** [go](url)",
            Some("# Heading\n\n_one_ __two__ [go](url)\n"),
            json!({"default":false,"MD026":true,"MD049":{"style":"underscore"},"MD050":{"style":"underscore"},"MD022":true,"MD012":true,"MD013":{"line_length":32},"MD047":true}),
        ),
    ] {
        for bom in [false, true] {
            let input = if bom {
                format!("\u{feff}{}", input.replace('\n', "\r\n"))
            } else {
                input.into()
            };
            let settings = config(rules.clone());
            let first = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
            if let Some(expected) = expected {
                let expected = if bom {
                    format!("\u{feff}{}", expected.replace('\n', "\r\n"))
                } else {
                    expected.into()
                };
                assert_eq!(first.output, expected);
            } else {
                assert!(first.output.contains("[go](#heading)"));
                assert!(!first.output.contains("Heading!"));
            }
            assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
            let second =
                format_with_config(&first.output, &FormatOptions::default(), &settings).unwrap();
            assert_eq!(second.output, first.output);
            assert!(second.edits.is_empty());
            let mut actual = String::new();
            let mut cursor = 0;
            for edit in &first.edits {
                actual.push_str(&input[cursor..edit.span.start]);
                actual.push_str(&edit.replacement);
                cursor = edit.span.end;
            }
            actual.push_str(&input[cursor..]);
            assert_eq!(actual, first.output);
            let nofix =
                format_with_config(&input, &FormatOptions { fix: false }, &settings).unwrap();
            assert_eq!(nofix.output, input);
            assert!(nofix.edits.is_empty());
            let nofix = format_with_config(
                &input,
                &FormatOptions::default(),
                &ResolvedConfig {
                    fix: false,
                    ..settings
                },
            )
            .unwrap();
            assert_eq!(nofix.output, input);
            assert!(nofix.edits.is_empty());
        }
    }
}

#[test]
fn existing_reverse_edit_priorities_remain_bounded_in_warning_only_cli_modes() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let errors = dir.path().join("errors.log");
    for (input, expected, rules) in [
        (
            "- # H\n",
            "- # H\n",
            json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD023":{"severity":"warning"}}),
        ),
        (
            "- - a\n",
            "-   -   a\n",
            json!({"default":false,"MD007":{"severity":"warning"},"MD030":{"ul_single":3,"severity":"warning"}}),
        ),
        (
            "> - a\n",
            "> - a\n",
            json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD027":{"severity":"warning"}}),
        ),
    ] {
        fs::write(&path, input).unwrap();
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"config":rules}).to_string(),
        )
        .unwrap();
        for _ in 0..2 {
            let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args(["--write", "doc.md"])
                .stdout(Stdio::null())
                .stderr(Stdio::from(fs::File::create(&errors).unwrap()))
                .spawn()
                .unwrap();
            let start = Instant::now();
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert_eq!(status.code(), Some(0));
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("real rule priority did not terminate");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), expected);
            assert!(!fs::read_to_string(&errors).unwrap().contains("Error"));
        }
    }
}
