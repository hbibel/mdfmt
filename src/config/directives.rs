use super::{
    ConfigError, ResolvedConfig, RuleConfig, RuleState, load::parse_jsonc, rules::matching_rules,
};
use crate::Source;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::LazyLock,
};

static START: LazyLock<Regex> = LazyLock::new(|| {
    let space = super::JS_SPACE;
    Regex::new(&format!(r"(<!--{space}*(?i-u:markdownlint-(disable|enable|capture|restore|disable-file|enable-file|disable-line|disable-next-line|configure-file)))(?:{space}|-->)")).unwrap()
});

#[derive(Debug)]
pub struct DocumentConfig {
    rules: BTreeMap<String, RuleState>,
    lines: Vec<BTreeSet<String>>,
    directives: Vec<Range<usize>>,
    line_overrides: Vec<BTreeSet<String>>,
}

struct Directive<'a> {
    action: String,
    parameter: &'a str,
    span: Range<usize>,
}

fn scan(text: &str) -> Vec<Directive<'_>> {
    let mut result = vec![];
    for matched in START.captures_iter(text) {
        let prefix = matched.get(1).unwrap();
        let Some(end) = text[prefix.end()..].find("-->") else {
            break;
        };
        let end = prefix.end() + end;
        result.push(Directive {
            action: matched[2].to_ascii_uppercase(),
            parameter: &text[prefix.end()..end],
            span: prefix.start()..end + 3,
        });
    }
    result
}

fn toggle(
    state: &mut BTreeSet<String>,
    directive: &Directive<'_>,
    all: &BTreeMap<String, RuleState>,
) {
    let targets: Vec<_> = if directive.parameter.trim_matches(super::js_space).is_empty() {
        all.keys().map(String::as_str).collect()
    } else {
        directive
            .parameter
            .split(super::js_space)
            .filter(|part| !part.is_empty())
            .flat_map(matching_rules)
            .collect()
    };
    let enabled = directive.action.starts_with("ENABLE");
    for target in targets {
        if enabled {
            state.insert(target.into());
        } else {
            state.remove(target);
        }
    }
}

fn parse_inline(parameter: &str) -> Result<RuleConfig, ConfigError> {
    let value = parse_jsonc(parameter)
        .or_else(|_| {
            toml::from_str::<toml::Value>(parameter)
                .map_err(|e| ConfigError::invalid("configure-file", e.to_string()))
                .and_then(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| ConfigError::invalid("configure-file", e.to_string()))
                })
        })
        .or_else(|_| {
            serde_yaml::from_str::<Value>(parameter)
                .map_err(|e| ConfigError::invalid("configure-file", e.to_string()))
        })?;
    RuleConfig::from_value(value)
}

impl ResolvedConfig {
    pub fn document(&self, source: &str) -> Result<DocumentConfig, ConfigError> {
        let normalized = source
            .strip_prefix('\u{feff}')
            .unwrap_or(source)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let mut rules = self.rules.clone();
        let all_directives = if self.no_inline_config {
            vec![]
        } else {
            scan(source)
        };
        for directive in &all_directives {
            if directive.action == "CONFIGURE-FILE" {
                rules.merge(&parse_inline(directive.parameter)?);
            }
        }
        let rules = rules.effective();
        let mut enabled: BTreeSet<_> = rules
            .iter()
            .filter(|(_, state)| state.enabled)
            .map(|(id, _)| id.clone())
            .collect();
        let per_line: Vec<_> = normalized
            .split('\n')
            .map(|line| {
                if self.no_inline_config {
                    vec![]
                } else {
                    scan(line)
                }
            })
            .collect();
        for directive in per_line.iter().flatten() {
            if matches!(directive.action.as_str(), "ENABLE-FILE" | "DISABLE-FILE") {
                toggle(&mut enabled, directive, &rules);
            }
        }
        let mut captured = enabled.clone();
        let mut lines = Vec::with_capacity(per_line.len());
        for directives in &per_line {
            for directive in directives {
                match directive.action.as_str() {
                    "CAPTURE" => captured = enabled.clone(),
                    "RESTORE" => enabled = captured.clone(),
                    "ENABLE" | "DISABLE" => toggle(&mut enabled, directive, &rules),
                    _ => {}
                }
            }
            lines.push(enabled.clone());
        }
        let mut line_overrides = vec![BTreeSet::new(); lines.len()];
        for (line, directives) in per_line.iter().enumerate() {
            for directive in directives {
                let offset = match directive.action.as_str() {
                    "DISABLE-LINE" => 0,
                    "DISABLE-NEXT-LINE" => 1,
                    _ => continue,
                };
                if let Some(state) = lines.get_mut(line + offset) {
                    let before = state.clone();
                    toggle(state, directive, &rules);
                    line_overrides[line + offset].extend(before.difference(state).cloned());
                }
            }
        }
        Ok(DocumentConfig {
            rules,
            lines,
            directives: all_directives.into_iter().map(|d| d.span).collect(),
            line_overrides,
        })
    }
}

impl DocumentConfig {
    pub(crate) fn has_any_line_override(&self, line: usize) -> bool {
        self.line_overrides
            .get(line - 1)
            .is_some_and(|rules| !rules.is_empty())
    }

    pub(crate) fn has_line_override(
        &self,
        rule: &str,
        span: Range<usize>,
        source: &Source,
    ) -> bool {
        let start = source.position(span.start).unwrap().line;
        let end = source.position(span.end).unwrap().line;
        (start..=end).any(|line| {
            self.line_overrides
                .get(line - 1)
                .is_some_and(|rules| rules.contains(rule))
        })
    }

    pub fn state(&self, rule: &str, line: usize) -> Option<RuleState> {
        let targets = matching_rules(rule);
        if targets.len() != 1 {
            return None;
        }
        let id = targets[0];
        let mut state = self.rules.get(id)?.clone();
        state.enabled = self.lines.get(line.checked_sub(1)?)?.contains(id);
        Some(state)
    }

    pub fn allows(&self, rule: &str, span: Range<usize>, source: &str) -> bool {
        self.allows_source(rule, span, &Source::new(source.into()))
    }

    pub(crate) fn allows_source(&self, rule: &str, span: Range<usize>, source: &Source) -> bool {
        if span.start > span.end
            || source.text().get(span.clone()).is_none()
            || self
                .directives
                .iter()
                .any(|d| span.start <= d.end && span.end >= d.start)
        {
            return false;
        }
        let Ok(start) = source.position(span.start) else {
            return false;
        };
        let last = if span.is_empty() {
            span.end
        } else {
            source.text()[..span.end]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i)
        };
        let end = source.position(last).or_else(|_| source.position(span.end));
        let Ok(end) = end else {
            return false;
        };
        (start.line..=end.line)
            .all(|line| self.state(rule, line).is_some_and(|state| state.enabled))
    }
}
