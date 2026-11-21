use mdfmt::{
    FormatOptions, ProcessingError,
    config::{FrontMatterPattern, ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
use std::{
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn settings(pattern: &str, rules: Value) -> ResolvedConfig {
    ResolvedConfig {
        front_matter: Some(FrontMatterPattern::new(pattern).unwrap()),
        no_inline_config: true,
        rules: RuleConfig::from_value(rules).unwrap(),
        ..Default::default()
    }
}

fn verify(input: &str, config: &ResolvedConfig, id: &str, protected: &[(usize, usize)]) -> bool {
    let checked = format_with_config(input, &FormatOptions { fix: false }, config)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
    assert_eq!(checked.output, input, "{id}: check changed source");
    assert!(checked.edits.is_empty(), "{id}: check emitted edits");
    let first = format_with_config(input, &FormatOptions::default(), config)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
    let mut rebuilt = input.to_owned();
    for edit in first.edits.iter().rev() {
        assert_eq!(edit.origin, "MD013", "{id}: unrelated repair");
        assert!(
            input[edit.span.clone()]
                .chars()
                .chain(edit.replacement.chars())
                .all(|ch| ch.is_ascii_whitespace() || ch == '>'),
            "{id}: non-layout payload changed"
        );
        assert!(
            protected.iter().all(|&(start, end)| {
                if edit.span.is_empty() {
                    edit.span.start <= start || end <= edit.span.start
                } else {
                    edit.span.end <= start || end <= edit.span.start
                }
            }),
            "{id}: protected source overlap {:?}",
            edit.span
        );
        rebuilt.replace_range(edit.span.clone(), &edit.replacement);
    }
    assert_eq!(rebuilt, first.output, "{id}: original edit reconstruction");
    let second = format_with_config(&first.output, &FormatOptions::default(), config)
        .unwrap_or_else(|error| panic!("{id}: repeated {error}"));
    assert_eq!(second.output, first.output, "{id}: not idempotent");
    assert!(second.edits.is_empty(), "{id}: repeated edits");
    for finding in &first.diagnostics {
        if let Some(span) = &finding.span {
            assert!(
                first.output.get(span.clone()).is_some(),
                "{id}: invalid diagnostic coordinate"
            );
        }
    }
    first.output != input
}

#[test]
fn licensed_corpus_preserves_upstream_atomic_ranges() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/corpus/observations.json");
    let rows: Value = serde_json::from_str(
        &fs::read_to_string(path).expect("offline corpus observations are required"),
    )
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 654);
    assert_eq!(
        rows.iter()
            .filter(|row| row["id"].as_str().unwrap().starts_with("commonmark-"))
            .count(),
        652
    );
    let config = settings(
        "^never-match$",
        json!({"default":false,"MD013":{"line_length":12}}),
    );
    let mut changed = 0;
    for row in rows {
        let protected: Vec<_> = row["protected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|span| {
                (
                    span[0].as_u64().unwrap() as usize,
                    span[1].as_u64().unwrap() as usize,
                )
            })
            .collect();
        changed += usize::from(verify(
            row["input"].as_str().unwrap(),
            &config,
            row["id"].as_str().unwrap(),
            &protected,
        ));
    }
    assert!(changed > 0, "corpus never exercised formatting");
}

#[test]
fn upstream_ranges_keep_original_bom_newline_container_bytes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/corpus/coordinate-observations.json");
    let rows: Value = serde_json::from_str(
        &fs::read_to_string(path).expect("offline upstream coordinate controls are required"),
    )
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 12);
    let config = settings(
        "^never-match$",
        json!({"default":false,"MD013":{"line_length":12}}),
    );
    for row in rows {
        let input = row["input"].as_str().unwrap();
        let protected: Vec<_> = row["protected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|span| {
                let start = span[0].as_u64().unwrap() as usize;
                let end = span[1].as_u64().unwrap() as usize;
                let original = input.get(start..end).expect("original UTF-8 boundaries");
                if span[2] == "link" {
                    assert_eq!(original, "[link](/url)");
                    assert_eq!(start, input.find("[link](/url)").unwrap());
                } else if span[2] == "codeText" {
                    assert_eq!(original, "`code`");
                }
                (start, end)
            })
            .collect();
        verify(input, &config, row["id"].as_str().unwrap(), &protected);
    }
}

fn protected_values(input: &str) -> Vec<(String, String)> {
    use markdown::mdast::Node;
    fn visit(node: &Node, input: &str, values: &mut Vec<(String, String)>) {
        let kind = match node {
            Node::Code(_) => Some("code"),
            Node::InlineCode(_) => Some("inline-code"),
            Node::Table(_) => Some("table"),
            Node::Link(_) => Some("link"),
            Node::Image(_) => Some("image"),
            _ => None,
        };
        if let Some(kind) = kind {
            let value = match node {
                Node::Code(code) => code.value.clone(),
                Node::InlineCode(code) => code.value.clone(),
                _ => {
                    let position = node.position().unwrap();
                    input[position.start.offset..position.end.offset].into()
                }
            };
            values.push((kind.into(), value));
        } else if let Some(children) = node.children() {
            for child in children {
                visit(child, input, values);
            }
        }
    }
    let parsed = markdown::to_mdast(input, &markdown::ParseOptions::gfm()).unwrap();
    let mut values = vec![];
    visit(&parsed, input, &mut values);
    values
}

fn seeded_cases() {
    let chunks = [
        "alpha beta gamma delta ",
        "*word* ",
        "**bold** ",
        "`code a b` ",
        "[link](https://example.test/a_b) ",
        "[ref][id] ",
        "\\*literal ",
        "\n\n",
        "> ",
        "- ",
        "\t",
        "😀 é K ſ ",
        "<!-- note --> ",
        "<span>raw</span> ",
        "| a | b |\n| - | - |\n",
        "```text\ncode untouched\n```\n",
        "&amp; &#10; ",
    ];
    let mut seed = 0x4d44464d54_u64;
    let mut changed = 0;
    let config = settings(
        r"^%%%\n[\s\S]*?\n%%%\n",
        json!({"default":false,"MD013":{"line_length":12}}),
    );
    for index in 0..256 {
        let mut input = if index % 16 == 0 {
            "%%%\n# Secret!\n%%%\n".into()
        } else {
            String::new()
        };
        for _ in 0..12 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            input.push_str(chunks[(seed as usize) % chunks.len()]);
        }
        input.push_str("\n\n[id]: /target\n");
        if index % 3 == 0 {
            input = input.replace('\n', "\r\n");
        } else if index % 3 == 1 {
            input = input.replace('\n', "\r");
        }
        if index % 5 == 0 {
            input.insert(0, '\u{feff}');
        }
        let before = protected_values(&input);
        changed += usize::from(verify(&input, &config, &format!("seed-{index}"), &[]));
        let result = format_with_config(&input, &FormatOptions::default(), &config).unwrap();
        assert_eq!(
            protected_values(&result.output),
            before,
            "seed-{index}: protected values changed"
        );
        let end = config
            .front_matter
            .as_ref()
            .unwrap()
            .prefix_end(&input)
            .unwrap();
        assert_eq!(
            &result.output[..end],
            &input[..end],
            "seed-{index}: metadata changed"
        );
    }
    assert!(changed > 0, "seeded cases never exercised formatting");
    eprintln!("seeded native cases: 256; transformed: {changed}");
    let hostile = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD051":{"ignored_pattern":"(a+)+b"}}),
        )
        .unwrap(),
        ..Default::default()
    };
    assert!(matches!(
        format_with_config(
            &format!("[x](#{})\n", "a".repeat(200)),
            &FormatOptions::default(),
            &hostile
        ),
        Err(ProcessingError::WorkLimit(_))
    ));
    let safe = settings(
        "^absent",
        json!({"default":false,"MD013":{"line_length":12}}),
    );
    assert_eq!(
        format_with_config("alpha beta gamma delta\n", &FormatOptions::default(), &safe)
            .unwrap()
            .output,
        "alpha beta\ngamma delta\n"
    );
}

#[test]
fn seeded_native_cases_are_bounded_and_repeatable() {
    if std::env::var_os("MDFMT_NATIVE_FUZZ_CHILD").is_some() {
        seeded_cases();
        return;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "seeded_native_cases_are_bounded_and_repeatable",
            "--nocapture",
        ])
        .env("MDFMT_NATIVE_FUZZ_CHILD", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.reopen().unwrap()))
        .stderr(Stdio::from(log.reopen().unwrap()))
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "{}",
                fs::read_to_string(log.path()).unwrap()
            );
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("seeded native cases exceeded 30 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn approved_repairs_remain_available_under_hardening() {
    let config = settings(
        "^absent",
        json!({"default":false,"MD026":true,"MD051":true}),
    );
    let first = format_with_config(
        "# Target!\n\n[case](#TARGET)\n",
        &FormatOptions::default(),
        &config,
    )
    .unwrap();
    assert_eq!(first.output, "# Target\n\n[case](#target)\n");
    let second = format_with_config(&first.output, &FormatOptions::default(), &config).unwrap();
    assert_eq!(second.output, first.output);
    assert!(second.edits.is_empty());
}
