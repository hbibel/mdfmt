use crate::{Diagnostic, Severity, Source, config::DocumentConfig};
use std::ops::Range;

pub(crate) struct RuleConflict {
    pub span: Range<usize>,
    pub rules: Vec<String>,
    pub retained: Option<String>,
}

pub(crate) fn diagnose_conflicts(
    source: &Source,
    document: &DocumentConfig,
    conflicts: &[RuleConflict],
) -> Vec<Diagnostic> {
    let mut result = vec![];
    for conflict in conflicts {
        let Some(position) = source.position(conflict.span.start).ok() else {
            continue;
        };
        let mut rules = conflict.rules.clone();
        rules.sort();
        rules.dedup();
        let states: Vec<_> = rules
            .iter()
            .filter_map(|rule| {
                document
                    .state(rule, position.line)
                    .filter(|state| state.enabled)
            })
            .collect();
        let error = if conflict.retained.is_none() {
            rules.iter().any(|rule| {
                (1..=source.line_count()).any(|line| {
                    document
                        .state(rule, line)
                        .is_some_and(|state| state.enabled && state.severity == Severity::Error)
                })
            })
        } else {
            states.iter().any(|state| state.severity == Severity::Error)
        };
        let severity = if error {
            Severity::Error
        } else {
            Severity::Warning
        };
        let outcome = conflict.retained.as_ref().map_or_else(
            || "original source preserved".into(),
            |rule| format!("retained {rule} priority"),
        );
        let diagnostic = Diagnostic {
            code: "rule-conflict".into(),
            severity,
            message: format!(
                "Conflicting formatting requirements: {}; {outcome}",
                rules.join(", ")
            ),
            rule_id: None,
            span: Some(conflict.span.clone()),
        };
        if !result.contains(&diagnostic) {
            result.push(diagnostic);
        }
    }
    result.sort_by_key(|diagnostic| diagnostic.span.as_ref().map(|span| (span.start, span.end)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Severity,
        config::{ResolvedConfig, RuleConfig},
    };
    use serde_json::json;

    #[test]
    fn cycle_severity_includes_rules_enabled_after_the_fallback_anchor() {
        let source =
            Source::new("<!-- markdownlint-disable-next-line MD023 -->\n- # H\n- # I\n".into());
        let config = ResolvedConfig {
            rules: RuleConfig::from_value(
                json!({"default":false,"MD007":{"severity":"warning"},"MD023":true}),
            )
            .unwrap(),
            ..Default::default()
        };
        let document = config.document(source.text()).unwrap();
        let at = source.line_range(2).unwrap().start;
        let diagnostics = diagnose_conflicts(
            &source,
            &document,
            &[RuleConflict {
                span: at..at,
                rules: vec!["MD007".into(), "MD023".into()],
                retained: None,
            }],
        );
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn conflict_severity_follows_contributing_rules_without_a_new_error_rule() {
        let source = Source::new("\u{feff}- # H\r\n".into());
        for (heading, severity) in [("warning", Severity::Warning), ("error", Severity::Error)] {
            let config=ResolvedConfig{rules:RuleConfig::from_value(json!({"default":false,"MD007":{"start_indented":true,"severity":"warning"},"MD023":{"severity":heading}})).unwrap(),..Default::default()};
            let document = config.document(source.text()).unwrap();
            let result = diagnose_conflicts(
                &source,
                &document,
                &[RuleConflict {
                    span: 3..3,
                    rules: vec!["MD007".into(), "MD023".into()],
                    retained: None,
                }],
            );
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].code, "rule-conflict");
            assert_eq!(result[0].rule_id, None);
            assert_eq!(result[0].severity, severity);
            assert_eq!(result[0].span, Some(3..3));
            assert!(result[0].message.contains("original source preserved"));
        }
    }
}
