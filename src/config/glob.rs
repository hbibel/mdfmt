use super::{ConfigError, brace, extglob};
use globset::GlobBuilder;
use regex::{Regex, RegexBuilder};

pub(crate) struct GlobMatcher {
    regex: Regex,
    original: Option<String>,
    pub(super) negated: bool,
}

impl GlobMatcher {
    pub(crate) fn is_match(&self, path: impl AsRef<str>) -> bool {
        let path = path.as_ref();
        // Picomatch's negation wrapper uses a non-dotall JavaScript .* expression.
        let before_final_break = path
            .strip_suffix("\r\n")
            .or_else(|| path.strip_suffix(['\n', '\r', '\u{2028}', '\u{2029}']))
            .unwrap_or(path);
        self.original.as_deref() == Some(path)
            || ((self.negated != self.regex.is_match(&utf16_units(path)))
                && (!self.negated
                    || !before_final_break.contains(['\n', '\r', '\u{2028}', '\u{2029}'])))
    }
}

struct Fragments {
    prefix: String,
    replacements: Vec<(String, String)>,
}

impl Fragments {
    fn new(pattern: &str) -> Self {
        let mut prefix = String::from("mdfmtGlobFragment");
        while pattern.contains(&prefix) {
            prefix.push('_');
        }
        Self {
            prefix,
            replacements: Vec::new(),
        }
    }

    fn insert(&mut self, regex: String) -> String {
        let marker = format!("{}{}End", self.prefix, self.replacements.len());
        self.replacements
            .push((marker.clone(), utf16_units(&regex)));
        marker
    }

    fn apply(self, mut regex: String) -> String {
        for (marker, replacement) in self.replacements {
            regex = regex.replace(&marker, &format!("(?:{replacement})"));
        }
        regex
    }
}

pub(crate) fn matcher(pattern: &str) -> Result<GlobMatcher, ConfigError> {
    build(pattern, false)
}

fn build(pattern: &str, filtering: bool) -> Result<GlobMatcher, ConfigError> {
    if pattern.encode_utf16().take(65_537).count() > 65_536 {
        return Err(ConfigError::invalid(
            pattern,
            "glob exceeds upstream's 65536-unit length limit",
        ));
    }
    let mut fragments = Fragments::new(pattern);
    let translated =
        extglob::translate(pattern, filtering, &mut || fragments.insert(String::new()))?;
    let original = filtering.then(|| pattern.to_owned());
    let negations = pattern.bytes().take_while(|b| *b == b'!').count();
    let negated = negations % 2 == 1;
    let mut pattern = &translated[negations..];
    while let Some(rest) = pattern.strip_prefix("./") {
        pattern = rest;
    }
    let expanded = expand_classes(pattern, filtering, &mut fragments)?;
    let expanded = brace::translate(&expanded, filtering, &mut |regex| fragments.insert(regex))?;
    let glob = GlobBuilder::new(&utf16_units(&expanded))
        .literal_separator(true)
        .backslash_escape(true)
        .empty_alternates(true)
        .build()
        .map_err(|e| ConfigError::invalid(pattern, e.to_string()))?;
    let regex = unicode_regex(glob.regex());
    let regex = fragments.apply(regex);
    let regex = RegexBuilder::new(&regex)
        .dot_matches_new_line(true)
        .build()
        .map_err(|e| ConfigError::invalid(pattern, e.to_string()))?;
    Ok(GlobMatcher {
        regex,
        original,
        negated,
    })
}

pub(crate) fn filter_matches(path: &str, patterns: &[String]) -> Result<bool, ConfigError> {
    let patterns = patterns
        .iter()
        .map(|p| build(p, true))
        .collect::<Result<Vec<_>, _>>()?;
    let mut included = patterns.iter().all(|p| p.negated);
    for pattern in patterns {
        if pattern.is_match(path) != pattern.negated {
            included = !pattern.negated;
        }
    }
    Ok(included)
}

fn utf16_units(text: &str) -> String {
    // Represent surrogate code units as distinct scalars so Rust regexes count like non-u JavaScript regexes.
    text.encode_utf16()
        .map(|unit| {
            char::from_u32(
                u32::from(unit)
                    + if (0xd800..=0xdfff).contains(&unit) {
                        0x10000
                    } else {
                        0
                    },
            )
            .unwrap()
        })
        .collect()
}

fn unicode_regex(regex: &str) -> String {
    // Globset emits byte-oriented regexes, including UTF-8 byte escapes for Unicode literals.
    let mut result = String::new();
    let mut rest = regex.strip_prefix("(?-u)").unwrap_or(regex);
    while !rest.is_empty() {
        if rest.starts_with("\\x")
            && rest.len() >= 4
            && u8::from_str_radix(&rest[2..4], 16).is_ok_and(|b| b >= 0x80)
        {
            let mut bytes = Vec::new();
            while rest.starts_with("\\x") && rest.len() >= 4 {
                let Ok(byte) = u8::from_str_radix(&rest[2..4], 16) else {
                    break;
                };
                bytes.push(byte);
                rest = &rest[4..];
            }
            result.push_str(
                std::str::from_utf8(&bytes)
                    .expect("globset encodes Unicode literals as UTF-8 bytes"),
            );
        } else {
            let ch = rest.chars().next().unwrap();
            result.push(ch);
            rest = &rest[ch.len_utf8()..];
            if ch == '\\'
                && let Some(ch) = rest.chars().next()
            {
                result.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
    }
    result
}

fn expand_classes(
    pattern: &str,
    filtering: bool,
    fragments: &mut Fragments,
) -> Result<String, ConfigError> {
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
            let mut posix = false;
            while let Some(ch) = chars.next() {
                if ch == '[' && chars.peek() == Some(&':') {
                    chars.next();
                    let mut name = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == ':' && chars.peek() == Some(&']') {
                            chars.next();
                            break;
                        }
                        name.push(ch);
                    }
                    let class = posix_class(&name).ok_or_else(|| {
                        ConfigError::new(
                            "unsupported-glob",
                            pattern,
                            "unknown POSIX character class",
                        )
                    })?;
                    body.push_str(class);
                    posix = true;
                    continue;
                }
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
            if posix {
                if !filtering && body.starts_with('!') {
                    body.replace_range(..1, "^");
                }
                let body = js_class_body(&body, pattern)?;
                result.push_str(&fragments.insert(format!("[{body}]")));
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
            let literal = brace::escape(&format!("[{body}]"));
            if body.contains([
                '-', '*', '+', '?', '.', '^', '$', '{', '}', '(', ')', '|', '[', ']', '\\',
            ]) || (!filtering && body.starts_with('!'))
            {
                result.push_str(&class);
            } else {
                result.push_str(&format!("{{{class},{literal}}}"));
            }
        } else {
            result.push(ch);
        }
    }
    Ok(result)
}

fn js_class_body(body: &str, pattern: &str) -> Result<String, ConfigError> {
    let mut result = String::new();
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                result.push(ch);
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                return Err(ConfigError::new(
                    "unsupported-glob",
                    pattern,
                    "ambiguous double-hyphen POSIX classes require additional compatibility work",
                ));
            }
            // These are literals in JavaScript classes, but set operators in Rust regexes.
            '&' | '~' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    Ok(result)
}

fn posix_class(name: &str) -> Option<&'static str> {
    Some(match name {
        "alnum" => "a-zA-Z0-9",
        "alpha" => "a-zA-Z",
        "ascii" => r"\x00-\x7f",
        "blank" => " \t",
        "cntrl" => r"\x00-\x1f\x7f",
        "digit" => "0-9",
        "graph" => r"\x21-\x7e",
        "lower" => "a-z",
        "print" => r"\x20-\x7e",
        "punct" => r"\x21-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e",
        "space" => " \t\r\n\u{b}\u{c}",
        "upper" => "A-Z",
        "word" => "A-Za-z0-9_",
        "xdigit" => "A-Fa-f0-9",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{filter_matches, matcher};
    use serde_json::{Value, json};

    #[test]
    fn range_filter_suffixes_are_diagnosed_instead_of_approximated() {
        let error = filter_matches("file1suffix.md", &["file{1..3}suffix.md".into()]).unwrap_err();
        assert_eq!(error.code, "unsupported-glob");
    }

    #[test]
    fn ambiguous_posix_class_difference_operators_are_rejected() {
        let error = matcher("[[:alpha:]--a].md")
            .err()
            .expect("ambiguous class must be rejected");
        assert_eq!(error.code, "unsupported-glob");
    }

    #[test]
    fn padded_ranges_cannot_allocate_unbounded_expanded_patterns() {
        let pattern = format!("file{{{:020000}..{:020000}}}.md", 1, 1000);
        let error = crate::config::brace::translate(&pattern, false, &mut |_| unreachable!())
            .expect_err("expanded pattern must be bounded before compiling its regex");
        assert_eq!(error.code, "invalid-config");
    }

    #[test]
    fn patterns_above_upstream_length_limit_are_rejected() {
        let error = matcher(&"a".repeat(65_537))
            .err()
            .expect("pattern must be bounded");
        assert_eq!(error.code, "invalid-config");
    }

    #[test]
    fn exact_one_extglobs_reject_unbalanced_and_excessively_nested_groups() {
        for pattern in ["@(a|b", "@(a(b)|c).md"] {
            let error = matcher(pattern)
                .err()
                .expect("unsafe group must be rejected");
            assert_eq!(error.code, "unsupported-glob");
        }
        let pattern = format!("{}a{}", "@(".repeat(65), ")".repeat(65));
        let error = matcher(&pattern).err().expect("nesting must be bounded");
        assert_eq!(error.code, "invalid-config");
    }

    #[test]
    fn exact_one_groups_do_not_create_pattern_negation() {
        for (pattern, included) in [("@(!a).md", "!a.md"), ("@()!.md", "!.md")] {
            let compiled = matcher(pattern).unwrap();
            assert!(compiled.is_match(included), "{pattern}");
            for excluded in ["a.md", "b.md", "a/x.md"] {
                assert!(!compiled.is_match(excluded), "{pattern}: {excluded}");
            }
            assert!(!filter_matches("b.md", &[pattern.into()]).unwrap());
        }
    }

    #[test]
    fn exact_one_group_boundaries_do_not_create_globstars() {
        for pattern in ["@(*)*/x.md", "*@(*)/x.md", "@(*)@(*)/x.md"] {
            let compiled = matcher(pattern).unwrap();
            assert!(compiled.is_match("a/x.md"), "{pattern}");
            for excluded in ["x.md", "a/b/x.md"] {
                assert!(!compiled.is_match(excluded), "{pattern}: {excluded}");
                assert!(!filter_matches(excluded, &[pattern.into()]).unwrap());
            }
        }
    }

    #[test]
    fn exact_one_post_group_quantifiers_are_diagnosed() {
        for pattern in ["@(a|b)?.md", "@(a|b)+.md"] {
            assert_eq!(
                matcher(pattern).err().expect("quantifier").code,
                "unsupported-glob"
            );
            assert_eq!(
                filter_matches("a.md", &[pattern.into()]).unwrap_err().code,
                "unsupported-glob"
            );
        }
    }

    #[test]
    fn exact_one_leading_question_mark_is_diagnosed() {
        let pattern = "@(?|a).md";
        assert_eq!(
            matcher(pattern).err().expect("leading question mark").code,
            "unsupported-glob"
        );
        assert_eq!(
            filter_matches("a.md", &[pattern.into()]).unwrap_err().code,
            "unsupported-glob"
        );
    }

    #[test]
    fn matches_pinned_pattern_observations() {
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/configuration/glob-cases.json"
        ))
        .unwrap();
        let observations: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/configuration/glob-observations.json"
        ))
        .unwrap();
        let mut differences = Vec::new();
        for observation in observations.as_array().unwrap() {
            let pattern = observation["pattern"].as_str().unwrap();
            if fixtures["unsupported"]
                .as_array()
                .is_some_and(|patterns| patterns.iter().any(|p| p == pattern))
            {
                assert_eq!(matcher(pattern).err().unwrap().code, "unsupported-glob");
                assert_eq!(
                    filter_matches("a.md", &[pattern.into()]).unwrap_err().code,
                    "unsupported-glob"
                );
                continue;
            }
            for context in ["enumeration", "filtering"] {
                let unsupported = if context == "filtering" {
                    "unsupportedFiltering"
                } else {
                    "unsupportedEnumeration"
                };
                if fixtures[unsupported]
                    .as_array()
                    .is_some_and(|patterns| patterns.iter().any(|p| p == pattern))
                {
                    let error = if context == "filtering" {
                        filter_matches("a.md", &[pattern.into()]).unwrap_err()
                    } else {
                        matcher(pattern).err().expect("unsupported enumeration")
                    };
                    assert_eq!(error.code, "unsupported-glob", "{pattern}");
                    continue;
                }
                let compiled = if context == "enumeration" {
                    Some(matcher(pattern).unwrap())
                } else {
                    None
                };
                let actual: Vec<_> = fixtures["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|path| {
                        let path = path.as_str().unwrap();
                        if context == "enumeration" {
                            compiled.as_ref().unwrap().is_match(path)
                        } else {
                            filter_matches(path, &[pattern.into()]).unwrap()
                        }
                    })
                    .collect();
                if json!(actual) != observation[context] {
                    differences.push(format!(
                        "{context} {pattern}: got {} expected {}",
                        json!(actual),
                        observation[context]
                    ));
                }
            }
        }
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }
}
