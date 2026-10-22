use super::{ConfigError, brace, picomatch::Pattern};

pub(crate) struct GlobMatcher {
    patterns: Vec<Pattern>,
    pub(super) negated: bool,
}

impl GlobMatcher {
    pub(crate) fn is_match(&self, path: impl AsRef<str>) -> Result<bool, ConfigError> {
        for pattern in &self.patterns {
            if pattern.regex_matches(path.as_ref())? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub(crate) fn matcher(pattern: &str) -> Result<GlobMatcher, ConfigError> {
    if pattern.encode_utf16().take(65_537).count() > 65_536 {
        return Err(ConfigError::invalid(
            pattern,
            "glob exceeds upstream's 65536-unit length limit",
        ));
    }
    let patterns = brace::enumerate(pattern)?
        .iter()
        .map(|value| Pattern::new(value, true))
        .collect::<Result<Vec<_>, _>>()?;
    let negated = patterns.iter().any(|pattern| pattern.negated);
    Ok(GlobMatcher { patterns, negated })
}

pub(crate) fn filter_matches(path: &str, patterns: &[String]) -> Result<bool, ConfigError> {
    let patterns = patterns
        .iter()
        .map(|pattern| Pattern::new(pattern, false))
        .collect::<Result<Vec<_>, _>>()?;
    let mut included = patterns.iter().all(|pattern| pattern.negated);
    for pattern in patterns {
        if pattern.filter_matches(path)? != pattern.negated {
            included = !pattern.negated;
        }
    }
    Ok(included)
}

pub(crate) fn parent(pattern: &str) -> String {
    let enclosed = pattern.ends_with(['}', ']']) && pattern.contains(['{', '[']);
    let mut value = format!("{pattern}{}a", if enclosed { "/" } else { "" });
    loop {
        let trimmed = value.trim_end_matches('/');
        value = match trimmed.rfind('/') {
            Some(0) => "/".into(),
            Some(index) => trimmed[..index].to_owned(),
            None => ".".into(),
        };
        if !parent_is_glob(&value) {
            break;
        }
        if matches!(value.as_str(), "/" | "." | "") {
            break;
        }
    }
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_some_and(|ch| "!*?|[](){}".contains(*ch)) {
            output.push(chars.next().unwrap());
        } else {
            output.push(ch);
        }
    }
    output
}

pub(crate) fn dynamic(pattern: &str) -> bool {
    use std::sync::OnceLock;
    if pattern.contains(['\\', '*', '?']) || pattern.starts_with('!') {
        return true;
    }
    static EXPRESSIONS: OnceLock<[regex::Regex; 3]> = OnceLock::new();
    if EXPRESSIONS
        .get_or_init(|| {
            [
                regex::Regex::new(r"\[[^\[]*\]").unwrap(),
                regex::Regex::new(r"(?:^|[^!*+?@])\([^(]*\|[^|]*\)").unwrap(),
                regex::Regex::new(r"[!*+?@]\([^(]*\)").unwrap(),
            ]
        })
        .iter()
        .any(|expression| expression.is_match(pattern))
    {
        return true;
    }
    pattern
        .find('{')
        .and_then(|start| {
            pattern[start..]
                .find('}')
                .map(|end| &pattern[start..start + end])
        })
        .is_some_and(|body| body.contains(',') || body.contains(".."))
}

fn parent_is_glob(pattern: &str) -> bool {
    let chars: Vec<_> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied().unwrap_or('\0');
        if ch == '!' && i == 0 || ch == '*' || next == '?' && "].+)".contains(ch) {
            return true;
        }
        if "@?!+*".contains(ch)
            && next == '('
            && chars[i + 2..]
                .iter()
                .take_while(|&&ch| !matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
                .any(|&ch| ch == ')')
        {
            return true;
        }
        if matches!(ch, '[' | '{') {
            return true;
        }
        if ch == '(' {
            let rest = &chars[i + 1..];
            if !rest.contains(&')') {
                return true;
            }
            if next == '?'
                && chars.get(i + 2).is_some_and(|ch| ":!=".contains(*ch))
                && chars.get(i + 3) != Some(&')')
            {
                return true;
            }
            if next != '|'
                && let Some(pipe) = rest.iter().position(|&ch| ch == '|')
                && rest.get(pipe + 1) != Some(&')')
                && rest[pipe + 1..].contains(&')')
            {
                return true;
            }
        }
        if ch == '\\' {
            i += 2;
            let close = match next {
                '{' => Some('}'),
                '[' => Some(']'),
                '(' => Some(')'),
                _ => None,
            };
            if let Some(close) = close
                && i < chars.len()
                && let Some(end) = chars[i..].iter().position(|&ch| ch == close)
            {
                i += end + 1;
            }
            if chars.get(i) == Some(&'!') {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

pub(crate) struct Traversal {
    segments: Vec<Pattern>,
    globstar: Option<usize>,
}

impl Traversal {
    pub(crate) fn new(pattern: &str) -> Result<Self, ConfigError> {
        let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
        let mut parts = Vec::new();
        let (mut start, mut parens, mut braces, mut brackets) = (0, 0usize, 0usize, 0usize);
        let (mut quote, mut escaped) = (false, false);
        for (index, ch) in pattern.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                quote = !quote;
                continue;
            }
            if quote {
                continue;
            }
            match ch {
                '[' => brackets += 1,
                ']' => brackets = brackets.saturating_sub(1),
                '(' if brackets == 0 => parens += 1,
                ')' if brackets == 0 => parens = parens.saturating_sub(1),
                '{' if brackets == 0 => braces += 1,
                '}' if brackets == 0 => braces = braces.saturating_sub(1),
                '/' if brackets == 0 && braces == 0 && parens == 0 => {
                    parts.push(&pattern[start..index]);
                    start = index + 1;
                }
                _ => {}
            }
        }
        parts.push(&pattern[start..]);
        let globstar = parts.iter().position(|part| part.contains("**"));
        let segments = parts
            .iter()
            .map(|part| Pattern::new(part, true))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { segments, globstar })
    }

    pub(crate) fn permits(&self, directory: &str) -> Result<bool, ConfigError> {
        let parts: Vec<_> = directory.split('/').collect();
        if self.globstar.is_some_and(|index| parts.len() > index) {
            return Ok(true);
        }
        if self.globstar.is_none() && parts.len() >= self.segments.len() {
            return Ok(false);
        }
        for (part, segment) in parts.iter().zip(&self.segments) {
            if !segment.regex_matches(part)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
