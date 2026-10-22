use super::ConfigError;

pub(super) fn translate(
    pattern: &str,
    filtering: bool,
    fragment: &mut impl FnMut(String) -> String,
) -> Result<String, ConfigError> {
    let mut output = String::new();
    let mut opens = Vec::new();
    let mut chars = pattern.chars().peekable();
    let mut expanded = 0;
    while let Some(ch) = chars.next() {
        match ch {
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
                    }
                    body = true;
                }
            }
            '\\' => {
                output.push(ch);
                if let Some(next) = chars.next() {
                    output.push(next);
                }
            }
            '{' => {
                opens.push((output.len(), false));
                output.push(ch);
            }
            ',' => {
                if let Some((_, comma)) = opens.last_mut() {
                    *comma = true;
                }
                output.push(ch);
            }
            '}' => {
                let Some((open, comma)) = opens.pop() else {
                    output.push(ch);
                    continue;
                };
                let body = &output[open + 1..];
                if body.contains("..") && !body.contains(['{', '}', ',']) {
                    let replacement = if filtering {
                        if chars.peek().is_some_and(|c| !"./*?{},|)".contains(*c)) {
                            return Err(ConfigError::new(
                                "unsupported-glob",
                                pattern,
                                "literal suffixes after filter ranges require upstream backtracking compatibility",
                            ));
                        }
                        fragment(filter_range(body))
                    } else {
                        expand(body, pattern, &mut expanded)?
                            .unwrap_or_else(|| fragment(filter_range(body)))
                    };
                    output.truncate(open);
                    output.push_str(&replacement);
                } else if !comma {
                    output.replace_range(open..open + 1, r"\{");
                    output.push_str(r"\}");
                } else {
                    output.push(ch);
                }
            }
            _ => output.push(ch),
        }
    }
    Ok(output)
}

fn filter_range(body: &str) -> String {
    let mut args: Vec<_> = body.split("..").collect();
    args.sort_unstable();
    let class = format!("[{}]", args.join("-"));
    // Picomatch compiles ranges as sorted character classes, not fill-range sequences.
    if regex::Regex::new(&class).is_ok() {
        class
    } else {
        args.iter()
            .map(|arg| regex::escape(arg))
            .collect::<Vec<_>>()
            .join("(?-s:..)")
    }
}

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
    } else if args[..2].iter().all(|s| s.len() == 1 && s.is_ascii()) {
        (
            i64::from(args[0].as_bytes()[0]),
            i64::from(args[1].as_bytes()[0]),
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
            escape(&char::from(value as u8).to_string())
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
