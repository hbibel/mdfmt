use mdfmt::{
    FormatOptions, Source,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

#[test]
fn code_padding_findings_never_cross_lines_or_change_data_line_permissions() {
    let settings = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD038":true})).unwrap(),
        ..Default::default()
    };
    for (input, spans) in [
        ("` \n x  `\n", vec![3..4, 5..7]),
        ("`  x \n `\n", vec![1..3, 4..5]),
        ("> ` \n>  x  `\n", vec![7..8, 9..11]),
        ("- ` \n   x  `\n", vec![7..8, 9..11]),
        ("` \r\n x  `\r\n", vec![4..5, 6..8]),
        ("`  x \r\n `\r\n", vec![1..3, 4..5]),
    ] {
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.span.clone().unwrap())
                .collect::<Vec<_>>(),
            spans,
            "{input:?}"
        );
        let source = Source::new(input.into());
        for diagnostic in &result.diagnostics {
            let span = diagnostic.span.as_ref().unwrap();
            assert_eq!(
                source.position(span.start).unwrap().line,
                source.position(span.end).unwrap().line
            );
        }
    }
    let directive = "<!-- markdownlint-disable-next-line MD038 -->\n";
    let input = format!("{directive}` \n x  `\n");
    let result = format_with_config(&input, &FormatOptions::default(), &settings).unwrap();
    assert_eq!(result.output, input);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.span.clone().unwrap())
            .collect::<Vec<_>>(),
        [
            directive.len() + 3..directive.len() + 4,
            directive.len() + 5..directive.len() + 7
        ]
    );
}

#[test]
fn dense_code_spans_do_not_repeatedly_expand_a_long_physical_line() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    for (prefix, rules) in [
        ("", json!({"default":false,"MD049":{"style":"asterisk"}})),
        ("", json!({"default":false,"MD038":true})),
        ("> ", json!({"default":false,"MD038":true})),
    ] {
        let input = format!("{prefix}{}{}\n", "`x` ".repeat(2000), "a".repeat(262144));
        fs::write(directory.path().join("doc.md"), &input).unwrap();
        fs::write(
            directory.path().join(".markdownlint-cli2.jsonc"),
            json!({"config":rules}).to_string(),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
            .current_dir(directory.path())
            .args(["--check", "doc.md"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if start.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("dense inline spans exceeded the prefix extraction work budget: {rules}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let result = child.wait_with_output().unwrap();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert_eq!(
            fs::read_to_string(directory.path().join("doc.md")).unwrap(),
            input
        );
    }
}
