use mdfmt::{
    FormatOptions, FormatResult, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};

fn variants(input: &str) -> [String; 3] {
    [
        input.to_owned(),
        format!("\u{feff}{}", input.replace('\n', "\r\n")),
        input.replace('\n', "\r"),
    ]
}
fn stable(input: &str, rules: Value) -> FormatResult {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    };
    let first = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(first.output, input);
    assert!(first.edits.is_empty());
    for fix in [false, true] {
        let again = format_with_config(&first.output, &FormatOptions { fix }, &settings).unwrap();
        assert_eq!(again.output, input);
        assert_eq!(again.diagnostics, first.diagnostics);
        assert!(again.edits.is_empty());
    }
    first
}
fn findings(input: &str, result: &FormatResult) -> Vec<(String, usize)> {
    let source = Source::new(input.into());
    let mut result: Vec<_> = result
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.code.clone(),
                source
                    .position(d.span.as_ref().unwrap().start)
                    .unwrap()
                    .line,
            )
        })
        .collect();
    result.sort();
    result
}

#[test]
fn recovered_code_keeps_owner_and_sibling_identity_for_logical_grouping() {
    for (body, expected) in [
        (
            "- item\n<span>\n>     $ cmd\n>\n>     output\n",
            vec![("MD046".into(), 3)],
        ),
        (
            "- item\n<span>\n>     $ cmd\n\n>     output\n",
            vec![
                ("MD014".into(), 3),
                ("MD046".into(), 3),
                ("MD046".into(), 5),
            ],
        ),
        (
            "- item\n<span>\n>     $ cmd\n> >\n>     output\n",
            vec![
                ("MD014".into(), 3),
                ("MD046".into(), 3),
                ("MD046".into(), 5),
            ],
        ),
    ] {
        for input in variants(body) {
            let result = stable(
                &input,
                json!({"default":false,"MD014":true,"MD046":{"style":"fenced"}}),
            );
            assert_eq!(findings(&input, &result), expected, "{input:?}");
        }
    }
}

#[test]
fn comment_prefixes_do_not_override_following_markdown_token_flags() {
    for (body, options, expected) in [
        (
            "<!-- comment --> `github`\n",
            json!({"code_blocks":false}),
            0,
        ),
        (
            "<!-- comment --> github\n",
            json!({"html_elements":false}),
            1,
        ),
        (
            "<!-- comment\n--> github\n",
            json!({"html_elements":false}),
            1,
        ),
        ("<!-- github -->\n", json!({"html_elements":false}), 0),
        (
            "> <!-- comment --> `github`\n",
            json!({"code_blocks":false}),
            0,
        ),
        (
            "- <!-- comment --> github\n",
            json!({"html_elements":false}),
            1,
        ),
    ] {
        for input in variants(body) {
            let mut options = options.clone();
            options["names"] = json!(["GitHub"]);
            let result = stable(&input, json!({"default":false,"MD044":options}));
            assert_eq!(
                result.diagnostics.len(),
                expected,
                "{input:?}: {:?}",
                result.diagnostics
            );
            for diagnostic in result.diagnostics {
                assert_eq!(diagnostic.code, "MD044");
                assert_eq!(&input[diagnostic.span.unwrap()], "github");
            }
        }
    }
}

#[test]
fn container_relative_blank_lines_end_recovered_html_and_keep_code_anchors() {
    for body in [
        "> item\n<span>\n>\n>     github\n",
        "> > item\n<span>\n> >\n> >     github\n",
        "> item\n<span>\n>\t\n>     github\n",
    ] {
        for input in variants(body) {
            let result = stable(
                &input,
                json!({"default":false,"MD044":{"names":["GitHub"],"code_blocks":false},"MD046":{"style":"fenced"}}),
            );
            assert_eq!(
                findings(&input, &result),
                vec![("MD046".into(), 4)],
                "{input:?}"
            );
            let commands = input.replace("github", "$ cmd");
            let result = stable(&commands, json!({"default":false,"MD014":true}));
            assert_eq!(
                findings(&commands, &result),
                vec![("MD014".into(), 4)],
                "{commands:?}"
            );
        }
    }
    let input = "> item\n<span>\n>\n>     github\n\n```text\ncode\n```\n";
    let result = stable(input, json!({"default":false,"MD046":true}));
    assert_eq!(findings(input, &result), vec![("MD046".into(), 6)]);
    assert!(
        result.diagnostics[0]
            .message
            .contains("Expected: indented; Actual: fenced")
    );
    let barrier = "> item\n<span>\n> >\n>     github\n";
    let result = stable(
        barrier,
        json!({"default":false,"MD044":{"names":["GitHub"],"code_blocks":false,"html_elements":false},"MD046":{"style":"fenced"}}),
    );
    assert_eq!(findings(barrier, &result), vec![("MD044".into(), 4)]);
}

#[test]
fn review_corrections_preserve_files_in_check_write_and_no_fix_modes() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    for (input, rules, count) in [
        (
            "- item\n<span>\n>     $ cmd\n>\n>     output\n",
            json!({"default":false,"MD014":true}),
            0,
        ),
        (
            "<!-- comment --> `github`\n",
            json!({"default":false,"MD044":{"names":["GitHub"],"code_blocks":false}}),
            0,
        ),
        (
            "<!-- comment --> github\n",
            json!({"default":false,"MD044":{"names":["GitHub"],"html_elements":false}}),
            1,
        ),
        (
            "> item\n<span>\n>\n>     github\n",
            json!({"default":false,"MD046":{"style":"fenced"}}),
            1,
        ),
    ] {
        let input = format!("\u{feff}{}", input.replace('\n', "\r\n"));
        fs::write(&path, &input).unwrap();
        for fix in [false, true] {
            fs::write(
                dir.path().join(".markdownlint-cli2.jsonc"),
                json!({"fix":fix,"config":rules}).to_string(),
            )
            .unwrap();
            for mode in ["--check", "--write"] {
                for _ in 0..2 {
                    let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                        .current_dir(dir.path())
                        .args([mode, "doc.md"])
                        .output()
                        .unwrap();
                    assert_eq!(result.status.code(), Some(i32::from(count > 0)));
                    assert_eq!(
                        String::from_utf8_lossy(&result.stderr)
                            .matches("Error MD")
                            .count(),
                        count
                    );
                    assert_eq!(fs::read_to_string(&path).unwrap(), input);
                }
            }
        }
    }
}
