use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::json;

fn variants(input: &str) -> [String; 3] {
    [
        input.into(),
        format!("\u{feff}{}", input.replace('\n', "\r\n")),
        input.replace('\n', "\r"),
    ]
}
fn settings() -> ResolvedConfig {
    ResolvedConfig {
        rules: RuleConfig::from_value(json!({"default":false,"MD051":true})).unwrap(),
        ..Default::default()
    }
}
fn unchanged(input: &str, missing: Option<&str>) {
    for input in variants(input) {
        let mut previous = None;
        for fix in [false, true, true] {
            let result = format_with_config(&input, &FormatOptions { fix }, &settings()).unwrap();
            assert_eq!(result.output, input, "{input:?}");
            assert!(result.edits.is_empty());
            assert_eq!(
                result.diagnostics.len(),
                usize::from(missing.is_some()),
                "{input:?}: {:?}",
                result.diagnostics
            );
            if let Some(missing) = missing {
                assert_eq!(&input[result.diagnostics[0].span.clone().unwrap()], missing);
                assert!(!result.diagnostics[0].message.contains("Expected:"));
            }
            if let Some(previous) = previous {
                assert_eq!(result.diagnostics, previous);
            }
            previous = Some(result.diagnostics);
        }
    }
}

#[test]
fn valid_line_fragments_keep_navigation_even_when_case_collides_with_targets() {
    for input in [
        "# L1\n\n[x](#L1)\n",
        "# L1-L20\n\n[x](#L1-L20)\n",
        "# L1C2-L3C4\n\n[x](#L1C2-L3C4)\n",
        "<a id=\"l1\"></a>\n\n[x](#L1)\n",
    ] {
        unchanged(input, None);
    }
    let input = "# L1C2\n\n[x](#L1C2)\n";
    let result = format_with_config(input, &FormatOptions::default(), &settings()).unwrap();
    assert_eq!(result.output, "# L1C2\n\n[x](#l1c2)\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn automatic_link_payloads_never_invent_heading_targets_or_repairs() {
    for automatic in [
        "<https://example.com>",
        "https://example.com",
        "www.example.com",
        "<user@example.com>",
        "user@example.com",
    ] {
        unchanged(
            &format!("# a {automatic} b\n\n[x](#a--b) [y](#A-HTTPSEXAMPLECOM-B)\n"),
            Some("[y](#A-HTTPSEXAMPLECOM-B)"),
        );
    }
    unchanged(
        "# <https://example.com>\n\n[x](#Httpsexamplecom)\n",
        Some("[x](#Httpsexamplecom)"),
    );
    unchanged(
        "# <https://example.com>\n# httpsexamplecom\n\n[x](#httpsexamplecom-1)\n",
        Some("[x](#httpsexamplecom-1)"),
    );
    unchanged(
        "# a [label](https://example.com) b\n\n[x](#a-label-b)\n",
        None,
    );
}

#[test]
fn lexical_atx_caption_spaces_survive_native_child_range_loss() {
    for (heading, valid, case) in [
        ("# # Title", "#-title", "#-TITLE"),
        ("# Title # #", "#title-", "#TITLE-"),
    ] {
        for prefix in ["", "> ", "- "] {
            unchanged(
                &format!("{prefix}{heading}\n\n[x]({valid}) [y](#Title)\n"),
                Some("[y](#Title)"),
            );
            let input = format!("{prefix}{heading}\n\n[x]({case})\n");
            let result =
                format_with_config(&input, &FormatOptions::default(), &settings()).unwrap();
            assert_eq!(result.output, input.replace(case, valid));
            assert!(result.diagnostics.is_empty());
            let again =
                format_with_config(&result.output, &FormatOptions::default(), &settings()).unwrap();
            assert_eq!(again.output, result.output);
            assert!(again.edits.is_empty());
        }
    }
    unchanged(
        "# # Title\n# Title\n\n[x](#title-1)\n",
        Some("[x](#title-1)"),
    );
}

#[test]
fn multiline_code_and_math_preserve_data_spaces_after_container_prefixes() {
    for marker in ["`", "$"] {
        for (spaces, valid) in [(" ", "#a-a-b-c"), ("  ", "#a-a--b-c")] {
            for (first, next, underline) in [
                ("", "", "---"),
                ("> ", "> ", "> ---"),
                ("- ", "  ", "  ---"),
            ] {
                let input = format!(
                    "{first}a {marker}a\n{next}{spaces}b{marker} c\n{underline}\n\n[x]({valid}) [y](#A-AB-C)\n"
                );
                unchanged(&input, Some("[y](#A-AB-C)"));
            }
        }
    }
    for marker in ["`", "$"] {
        unchanged(
            &format!("> - a {marker}a\n>  b{marker} c\n>   ---\n\n[x](#a-a-b-c)\n"),
            None,
        );
        unchanged(
            &format!("> a {marker}a\n b{marker} c\n> ---\n\n[x](#a-a-b-c)\n"),
            None,
        );
        unchanged(
            &format!("a {marker} \n {marker} c\n---\n\n[x](#a----c)\n"),
            None,
        );
        unchanged(
            &format!("a {marker}\n a \n{marker} c\n---\n\n[x](#a--a--c)\n"),
            None,
        );
    }
}

#[test]
fn valid_line_fragment_cli_checks_and_writes_are_byte_preserving() {
    use std::{fs, process::Command};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let input = "\u{feff}# L1\r\n\r\n[x](#L1)\r\n";
    fs::write(&path, input).unwrap();
    for fix in [false, true] {
        fs::write(
            dir.path().join(".markdownlint-cli2.jsonc"),
            json!({"fix":fix,"config":{"default":false,"MD051":true}}).to_string(),
        )
        .unwrap();
        for mode in ["--check", "--write"] {
            for _ in 0..2 {
                let result = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                    .current_dir(dir.path())
                    .args([mode, "doc.md"])
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.code(),
                    Some(0),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert!(result.stderr.is_empty());
                assert_eq!(fs::read_to_string(&path).unwrap(), input);
            }
        }
    }
}
