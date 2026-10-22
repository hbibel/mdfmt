use mdfmt::{
    Severity,
    config::{ResolvedConfig, RuleConfig},
};
use serde_json::json;

#[test]
fn file_line_and_capture_passes_match_upstream_order() {
    let text = "one\n<!-- markdownlint-disable-file MD013 -->\n<!-- markdownlint-enable MD013 -->\n<!-- markdownlint-capture -->\n<!-- markdownlint-disable line-length -->\nsix\n<!-- markdownlint-restore -->\neight <!-- markdownlint-disable-line MD013 -->\n<!-- markdownlint-disable-next-line MD013 -->\nten\neleven\n";
    let document = ResolvedConfig::default().document(text).unwrap();
    let enabled: Vec<_> = (1..=11)
        .map(|line| document.state("MD013", line).unwrap().enabled)
        .collect();
    assert_eq!(
        enabled,
        [
            false, false, true, true, false, false, true, false, true, false, true
        ]
    );
    assert!(!document.allows("MD013", 0..3, text));
    assert!(!document.allows("MD013", 0..text.len(), text));
    let start = text.find("eleven").unwrap();
    assert!(document.allows("MD013", start..start + 6, text));
}

#[test]
fn configure_file_is_global_multiline_ordered_and_severity_preserving() {
    let text = "\u{feff}before\r\n<!-- markdownlint-configure-file {\r\n\"MD013\": {\"line_length\": 32, \"severity\": \"warning\"}\r\n} -->\r\nafter\r\n";
    let document = ResolvedConfig::default().document(text).unwrap();
    let state = document.state("line-length", 1).unwrap();
    assert!(state.enabled);
    assert_eq!(state.severity, Severity::Warning);
    assert_eq!(state.options["line_length"], 32);
    let source = "<!-- markdownlint-configure-file {\"MD013\":false} -->\n<!-- markdownlint-enable-file MD013 -->\n<!-- markdownlint-disable MD013 -->";
    let document = ResolvedConfig::default().document(source).unwrap();
    assert!(document.state("MD013", 1).unwrap().enabled);
    assert!(!document.state("MD013", 3).unwrap().enabled);
}

#[test]
fn raw_source_scan_includes_code_and_front_matter_but_can_be_disabled() {
    let text = "---\n<!-- markdownlint-disable-file MD013 -->\n---\n```\n<!-- markdownlint-disable MD026 -->\n```\n# Heading!\n";
    let config = ResolvedConfig::default();
    let document = config.document(text).unwrap();
    assert!(!document.state("MD013", 1).unwrap().enabled);
    assert!(!document.state("MD026", 7).unwrap().enabled);
    let ignored = ResolvedConfig {
        no_inline_config: true,
        ..config
    }
    .document(text)
    .unwrap();
    assert!(ignored.state("MD013", 1).unwrap().enabled);
    assert!(ignored.state("MD026", 7).unwrap().enabled);
}

#[test]
fn malformed_configure_file_errors_unless_inline_config_is_disabled() {
    let text = "<!-- markdownlint-configure-file {bad -->\nprose";
    assert_eq!(
        ResolvedConfig::default().document(text).unwrap_err().code,
        "invalid-config"
    );
    assert!(
        ResolvedConfig {
            no_inline_config: true,
            ..Default::default()
        }
        .document(text)
        .is_ok()
    );
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(json!({"MD013": false})).unwrap(),
        ..Default::default()
    };
    let document = config
        .document("text <!-- markdownlint-enable MD013 -->")
        .unwrap();
    assert!(document.state("MD013", 1).unwrap().enabled);
}
