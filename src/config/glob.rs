use super::ConfigError;
use globset::{GlobBuilder, GlobMatcher};

pub(crate) fn matcher(pattern: &str) -> Result<GlobMatcher, ConfigError> {
    build(pattern, false)
}

fn build(pattern: &str, filtering: bool) -> Result<GlobMatcher, ConfigError> {
    if ["!(", "@(", "+(", "?(", "*(", "[:"]
        .iter()
        .any(|p| pattern.contains(p))
        || brace_range(pattern)
        || pattern.starts_with("!!")
    {
        return Err(ConfigError::new(
            "unsupported-glob",
            pattern,
            "extglobs, brace ranges, repeated negation and POSIX classes require additional compatibility work",
        ));
    }
    let mut pattern = pattern;
    while let Some(rest) = pattern.strip_prefix("./") {
        pattern = rest;
    }
    let expanded = expand_classes(pattern, filtering);
    GlobBuilder::new(&expanded)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map(|g| g.compile_matcher())
        .map_err(|e| ConfigError::invalid(pattern, e.to_string()))
}

pub(crate) fn filter_matches(path: &str, patterns: &[String]) -> Result<bool, ConfigError> {
    let mut included = !patterns.iter().any(|p| !p.starts_with('!'));
    for pattern in patterns {
        matcher(pattern)?;
        let (negative, pattern) = pattern
            .strip_prefix('!')
            .map_or((false, pattern.as_str()), |p| (true, p));
        if build(pattern, true)?.is_match(path) {
            included = !negative;
        }
    }
    Ok(included)
}

fn brace_range(pattern: &str) -> bool {
    let mut depth = 0usize;
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            '.' if depth > 0 && chars.peek() == Some(&'.') => return true,
            _ => {}
        }
    }
    false
}

fn expand_classes(pattern: &str, filtering: bool) -> String {
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            result.push(ch);
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else if ch == '[' {
            let mut body = String::new();
            let mut closed = false;
            for ch in chars.by_ref() {
                if ch == ']' && !body.is_empty() {
                    closed = true;
                    break;
                }
                body.push(ch);
            }
            if !closed {
                result.push('[');
                result.push_str(&body);
                continue;
            }
            let class = if filtering && body.starts_with('!') {
                // micromatch treats ! as a class member; fast-glob treats it as class negation.
                if body.len() == 1 {
                    r"\!".into()
                } else {
                    format!("{{!,[{}]}}", &body[1..])
                }
            } else {
                format!("[{body}]")
            };
            let literal: String = format!("[{body}]")
                .chars()
                .flat_map(|ch| {
                    if "\\*?[]{},".contains(ch) {
                        vec!['\\', ch]
                    } else {
                        vec![ch]
                    }
                })
                .collect();
            result.push_str(&format!("{{{class},{literal}}}"));
        } else {
            result.push(ch);
        }
    }
    result
}
