use super::ConfigError;

struct Group {
    start: usize,
    brace_depth: usize,
    alternatives: bool,
}

pub(super) fn translate(
    pattern: &str,
    filtering: bool,
    boundary: &mut impl FnMut() -> String,
) -> Result<String, ConfigError> {
    let unsupported = |message| ConfigError::new("unsupported-glob", pattern, message);
    let mut output = String::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut brace_depth = 0;
    let mut has_extglob = false;
    let mut has_range = false;
    let mut has_posix_class = false;
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                output.push(ch);
                if let Some(next) = chars.next() {
                    output.push(next);
                }
            }
            '[' => {
                output.push(ch);
                let mut body = false;
                while let Some(ch) = chars.next() {
                    output.push(ch);
                    if ch == ']' && body {
                        break;
                    }
                    if ch == '\\'
                        && let Some(next) = chars.next()
                    {
                        output.push(next);
                    } else if ch == '[' && chars.peek() == Some(&':') {
                        has_posix_class = true;
                        for ch in chars.by_ref() {
                            output.push(ch);
                            if ch == ']' {
                                break;
                            }
                        }
                    }
                    body = true;
                }
            }
            '@' if chars.peek() == Some(&'(') => {
                has_extglob = true;
                chars.next();
                if chars.peek() == Some(&'?') {
                    return Err(unsupported(
                        "leading question marks in extglobs require upstream parsing compatibility",
                    ));
                }
                if groups.len() >= 64 {
                    return Err(ConfigError::invalid(
                        pattern,
                        "extglob nesting limit exceeded",
                    ));
                }
                // Empty regex fragments keep separate wildcard tokens from becoming a globstar.
                output.push_str(&boundary());
                groups.push(Group {
                    start: output.len(),
                    brace_depth,
                    alternatives: false,
                });
                output.push('{');
            }
            '!' | '+' | '?' | '*' if chars.peek() == Some(&'(') => {
                return Err(unsupported(
                    "repetition and negative extglobs require additional compatibility work",
                ));
            }
            '*' if !groups.is_empty() && chars.peek() == Some(&'*') => {
                return Err(unsupported(
                    "globstars inside extglobs require additional compatibility work",
                ));
            }
            '/' if !filtering && !groups.is_empty() => {
                return Err(unsupported(
                    "path separators inside enumerated extglobs require upstream traversal compatibility",
                ));
            }
            '(' if !groups.is_empty() => {
                return Err(unsupported(
                    "plain parentheses inside extglobs require additional compatibility work",
                ));
            }
            ')' if !groups.is_empty() => {
                if chars.peek().is_some_and(|ch| matches!(ch, '?' | '+')) {
                    return Err(unsupported(
                        "post-group quantifiers require additional compatibility work",
                    ));
                }
                let group = groups.pop().unwrap();
                if brace_depth != group.brace_depth {
                    return Err(unsupported(
                        "overlapping braces and extglobs are unsupported",
                    ));
                }
                if group.alternatives {
                    output.push('}');
                } else {
                    output.remove(group.start);
                }
                output.push_str(&boundary());
            }
            '|' if !groups.is_empty() => {
                let group = groups.last_mut().unwrap();
                if brace_depth != group.brace_depth {
                    return Err(unsupported(
                        "extglob alternatives inside braces require additional compatibility work",
                    ));
                }
                group.alternatives = true;
                output.push(',');
            }
            ',' if groups.last().is_some_and(|g| g.brace_depth == brace_depth) => {
                output.push_str(r"\,");
            }
            '{' => {
                brace_depth += 1;
                output.push(ch);
            }
            '}' => {
                if groups.last().is_some_and(|g| g.brace_depth >= brace_depth) {
                    return Err(unsupported(
                        "overlapping braces and extglobs are unsupported",
                    ));
                }
                brace_depth = brace_depth.saturating_sub(1);
                output.push(ch);
            }
            '.' if brace_depth > 0 && chars.peek() == Some(&'.') => {
                has_range = true;
                output.push(ch);
            }
            _ => output.push(ch),
        }
    }
    if !groups.is_empty() {
        return Err(unsupported("unclosed extglob group"));
    }
    if filtering && has_extglob && has_range {
        return Err(unsupported(
            "brace ranges combined with extglob filters require upstream backtracking compatibility",
        ));
    }
    if has_extglob && has_posix_class {
        return Err(unsupported(
            "POSIX classes combined with extglobs require upstream backtracking compatibility",
        ));
    }
    Ok(output)
}
