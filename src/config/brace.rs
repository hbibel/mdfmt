use super::ConfigError;

fn expand(body: &str, pattern: &str, expanded: &mut usize) -> Result<Option<String>, ConfigError> {
    let args: Vec<_> = body.split("..").collect();
    if !(2..=3).contains(&args.len()) {
        return Ok(None);
    }
    let integer = |s: &str| {
        let digits = s.strip_prefix('-').unwrap_or(s);
        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
    };
    let numeric = integer(args[0]) && integer(args[1]);
    let parse = |s: &str| {
        s.parse::<i64>()
            .ok()
            .filter(|n| n.unsigned_abs() <= 9_007_199_254_740_991)
            .ok_or_else(|| ConfigError::invalid(pattern, "brace range exceeds safe integer bounds"))
    };
    let (start, end) = if numeric {
        (parse(args[0])?, parse(args[1])?)
    } else if args[..2].iter().all(|s| s.encode_utf16().count() == 1) {
        (
            i64::from(args[0].chars().next().unwrap() as u32),
            i64::from(args[1].chars().next().unwrap() as u32),
        )
    } else {
        return Ok(None);
    };
    let step = if let Some(step) = args.get(2) {
        if !integer(step) {
            return Ok(None);
        }
        parse(step)?.unsigned_abs().max(1)
    } else {
        1
    };
    let count = start.abs_diff(end) / step + 1;
    if count > 1000 {
        return Err(ConfigError::invalid(
            pattern,
            "brace range expansion limit exceeded",
        ));
    }
    let padded = numeric
        && args.iter().any(|s| {
            let s = s.strip_prefix('-').unwrap_or(s);
            s.len() > 1 && s.starts_with('0')
        });
    let width = if padded {
        args.iter().map(|s| s.len()).max().unwrap()
    } else {
        0
    };
    let allocation = (count as usize).saturating_mul(width.max(20).saturating_add(1));
    if expanded.saturating_add(allocation) > 1_048_576 {
        return Err(ConfigError::invalid(
            pattern,
            "brace expansion size limit exceeded",
        ));
    }
    *expanded += allocation;
    let mut values = Vec::with_capacity(count as usize);
    for index in 0..count {
        let offset = (index * step) as i64;
        let value = if start <= end {
            start + offset
        } else {
            start - offset
        };
        values.push(if numeric {
            if value < 0 {
                format!(
                    "-{:0width$}",
                    value.unsigned_abs(),
                    width = width.saturating_sub(1)
                )
            } else {
                format!("{value:0width$}")
            }
        } else {
            escape(
                &char::from_u32(value as u32)
                    .ok_or_else(|| {
                        ConfigError::invalid(
                            pattern,
                            "brace range contains unpaired surrogate units",
                        )
                    })?
                    .to_string(),
            )
        });
    }
    Ok(Some(format!("{{{}}}", values.join(","))))
}

pub(super) fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if "\\*?[]{},".contains(ch) {
                vec!['\\', ch]
            } else {
                vec![ch]
            }
        })
        .collect()
}

pub(super) fn enumerate(pattern: &str) -> Result<Vec<String>, ConfigError> {
    let mut budget = 0;
    let mut ranges = 0;
    let result = enumerate_inner(pattern, 0, &mut budget, &mut ranges)?;
    let mut seen = std::collections::HashSet::new();
    let has_braces = pattern
        .find('{')
        .is_some_and(|index| pattern[index..].contains('}'));
    Ok(result
        .into_iter()
        .map(|value| {
            if has_braces {
                strip_quotes(&value)
            } else {
                value
            }
        })
        .filter(|value| seen.insert(value.clone()))
        .collect())
}

fn enumerate_inner(
    pattern: &str,
    depth: usize,
    budget: &mut usize,
    ranges: &mut usize,
) -> Result<Vec<String>, ConfigError> {
    if depth > 64 {
        return Err(ConfigError::invalid(
            pattern,
            "brace nesting limit exceeded",
        ));
    }
    let mut state = BraceScan::default();
    let mut open = None;
    let mut nesting = 0usize;
    let chars: Vec<_> = pattern.char_indices().collect();
    for (position, &(index, ch)) in chars.iter().enumerate() {
        if !state.active(ch, chars.get(position + 1).map(|(_, ch)| *ch)) {
            continue;
        }
        if ch == '{' {
            if open.is_none() {
                open = Some(index);
            }
            nesting += 1;
        }
        if ch != '}' || nesting == 0 {
            continue;
        }
        nesting -= 1;
        if nesting != 0 {
            continue;
        }
        let open = open.unwrap();
        let body = &pattern[open + 1..index];
        let parts = brace_parts(body);
        let alternatives = if parts.len() > 1 {
            let mut alternatives = Vec::new();
            for part in parts {
                alternatives.extend(enumerate_inner(part, depth + 1, budget, ranges)?);
            }
            alternatives
        } else if !body.contains(['{', '}', ',']) && body.contains("..") {
            if let Some(expanded) = expand(body, pattern, ranges)? {
                expanded[1..expanded.len() - 1]
                    .split(',')
                    .map(str::to_owned)
                    .collect()
            } else {
                vec![format!("{{{body}}}")]
            }
        } else {
            enumerate_inner(body, depth + 1, budget, ranges)?
                .into_iter()
                .map(|part| format!("{{{part}}}"))
                .collect()
        };
        let suffixes = enumerate_inner(&pattern[index + 1..], depth + 1, budget, ranges)?;
        let mut output = Vec::new();
        for alternative in alternatives {
            for suffix in &suffixes {
                let length = open
                    .saturating_add(alternative.len())
                    .saturating_add(suffix.len());
                *budget = budget.saturating_add(length);
                if *budget > 1_048_576 || output.len() >= 4096 {
                    return Err(ConfigError::invalid(
                        pattern,
                        "brace expansion allocation limit exceeded",
                    ));
                }
                output.push(format!("{}{alternative}{suffix}", &pattern[..open]));
            }
        }
        return Ok(output);
    }
    Ok(vec![pattern.into()])
}

fn brace_parts(body: &str) -> Vec<&str> {
    let mut state = BraceScan::default();
    let mut parts = Vec::new();
    let (mut braces, mut parens, mut start) = (0usize, 0usize, 0usize);
    let chars: Vec<_> = body.char_indices().collect();
    for (position, &(index, ch)) in chars.iter().enumerate() {
        if !state.active(ch, chars.get(position + 1).map(|(_, ch)| *ch)) {
            continue;
        }
        match ch {
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            ',' if braces == 0 && parens == 0 => {
                parts.push(&body[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
}

#[derive(Default)]
struct BraceScan {
    escaped: bool,
    quote: Option<char>,
    brackets: usize,
}

impl BraceScan {
    fn active(&mut self, ch: char, next: Option<char>) -> bool {
        if self.escaped {
            self.escaped = false;
            return false;
        }
        if ch == '\\' {
            self.escaped = true;
            return false;
        }
        if let Some(quote) = self.quote {
            if ch == quote {
                self.quote = None;
            }
            return false;
        }
        if self.brackets == 0 && matches!(ch, '"' | '\'' | '`') {
            self.quote = Some(ch);
            return false;
        }
        if ch == '[' && (self.brackets == 0 || next == Some(':')) {
            self.brackets += 1;
            return false;
        }
        if ch == ']' && self.brackets > 0 {
            self.brackets -= 1;
            return false;
        }
        self.brackets == 0
    }
}

fn strip_quotes(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    let mut state = BraceScan::default();
    while let Some(ch) = chars.next() {
        let before = state.quote;
        state.active(ch, chars.peek().copied());
        if before != state.quote {
            continue;
        }
        result.push(ch);
    }
    result
}
