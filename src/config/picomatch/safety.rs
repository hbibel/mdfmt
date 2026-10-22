use super::compile::escape;

pub(super) enum Repetition {
    Safe,
    Literal,
    Collapsed(String),
}

pub(super) fn branches(input: &str) -> Vec<String> {
    let mut result = vec![String::new()];
    let (mut bracket, mut paren, mut quote, mut escaped) = (0usize, 0usize, false, false);
    for ch in input.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quote = !quote;
        } else if !quote {
            match ch {
                '[' => bracket += 1,
                ']' => bracket = bracket.saturating_sub(1),
                '(' if bracket == 0 => paren += 1,
                ')' if bracket == 0 => paren = paren.saturating_sub(1),
                '|' if bracket == 0 && paren == 0 => {
                    result.push(String::new());
                    continue;
                }
                _ => {}
            }
        }
        result.last_mut().unwrap().push(ch);
    }
    result
}

fn trim(input: &str) -> &str {
    input.trim_matches(|ch: char| ch.is_whitespace() && ch != '\u{85}' || ch == '\u{feff}')
}

fn simple(input: &str) -> Option<String> {
    let mut value = trim(input);
    while value.starts_with("@(")
        && value.ends_with(')')
        && !value[2..value.len() - 1].contains(['\\', '(', ')', '[', ']', '{', '}', '|'])
    {
        value = &value[2..value.len() - 1];
    }
    let mut result = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else if "?*+@!()[]{}".contains(ch) {
            return None;
        } else {
            result.push(ch);
        }
    }
    Some(result)
}

fn repeated(input: &str, require_end: bool) -> Option<(char, &str, usize)> {
    let operator = input.chars().next()?;
    if !matches!(operator, '+' | '*') || !input[1..].starts_with('(') {
        return None;
    }
    let (mut bracket, mut paren, mut quote, mut escaped) = (0usize, 0usize, false, false);
    for (i, ch) in input.char_indices().skip(1) {
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
            '[' => bracket += 1,
            ']' if bracket > 0 => bracket -= 1,
            '(' if bracket == 0 => paren += 1,
            ')' if bracket == 0 && paren > 0 => {
                paren -= 1;
                if paren == 0 {
                    if require_end && i + 1 != input.len() {
                        return None;
                    }
                    return Some((operator, &input[2..i], i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn star_sequence(input: &str) -> Option<String> {
    let mut rest = input;
    let mut values = Vec::new();
    while !rest.is_empty() {
        let ('*', body, end) = repeated(rest, false)? else {
            return None;
        };
        let parts = branches(body);
        if parts.len() != 1 {
            return None;
        }
        let value = simple(&parts[0])?;
        if value.encode_utf16().count() != 1 {
            return None;
        }
        values.push(value);
        rest = &rest[end..];
    }
    if values.is_empty() {
        None
    } else if values.len() == 1 {
        Some(format!("{}*", escape(&values[0])))
    } else {
        Some(format!(
            "[{}]*",
            values.iter().map(|v| escape(v)).collect::<String>()
        ))
    }
}

pub(super) fn analyze(body: &str) -> Repetition {
    let parts: Vec<_> = branches(body)
        .into_iter()
        .map(|s| trim(&s).to_owned())
        .collect();
    if parts.len() > 1 {
        if parts
            .iter()
            .any(|s| s.is_empty() || s.chars().all(|ch| matches!(ch, '*' | '?')))
        {
            return Repetition::Literal;
        }
        let values: Vec<_> = parts
            .iter()
            .filter_map(|s| simple(s))
            .filter(|s| !s.is_empty())
            .collect();
        for (i, a) in values.iter().enumerate() {
            let ch = a.chars().next().unwrap();
            if ch.len_utf16() != 1 || !a.chars().all(|c| c == ch) {
                continue;
            }
            if values[i + 1..]
                .iter()
                .any(|b| b.chars().all(|c| c == ch) && (a.starts_with(b) || b.starts_with(a)))
            {
                return Repetition::Literal;
            }
        }
    }
    for part in parts {
        if let Some(output) = star_sequence(&part) {
            return Repetition::Collapsed(output);
        }
        if repeated(&part, true).is_some() {
            return Repetition::Literal;
        }
    }
    Repetition::Safe
}
