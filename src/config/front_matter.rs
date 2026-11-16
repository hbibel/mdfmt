use super::{
    ConfigError, ResolvedConfig,
    pattern::{JsPattern, PatternFlags},
};
use crate::{ProcessingError, Source};
use std::{iter::Peekable, sync::Arc};

fn escape(value: char, class: bool) -> Option<&'static str> {
    Some(match value {
        's' => r"[\t\n\x0c\r ]",
        'S' => r"[^\t\n\x0c\r ]",
        'd' => "[0-9]",
        'D' => "[^0-9]",
        'w' => "[A-Za-z0-9_]",
        'W' => "[^A-Za-z0-9_]",
        'v' => r"\x0b",
        '0' => r"\x00",
        'b' if !class => r"(?-u:\b)",
        'B' if !class => r"(?-u:\B)",
        _ => return None,
    })
}

fn invalid(message: impl Into<String>) -> ConfigError {
    ConfigError::invalid("frontMatter", message)
}

fn escaped_atom(
    chars: &mut Peekable<impl Iterator<Item = char>>,
    class: bool,
) -> Result<String, ConfigError> {
    let Some(value) = chars.next() else {
        return Ok("\\".into());
    };
    if value == 'c' && chars.peek().is_some_and(char::is_ascii_alphabetic) {
        return Ok(format!("\\x{:02x}", u32::from(chars.next().unwrap()) % 32));
    }
    if matches!(value, 'p' | 'P') {
        if chars.next() != Some('{') {
            return Err(invalid("unsupported Unicode property syntax"));
        }
        let name: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
        let property = super::front_matter_properties::class(&name).ok_or_else(|| {
            invalid(format!(
                "unsupported or invalid pinned RE2JS property: {name}"
            ))
        })?;
        return Ok(if value == 'P' {
            format!("[^{property}]")
        } else {
            property.into()
        });
    }
    Ok(escape(value, class)
        .map(String::from)
        .unwrap_or_else(|| format!("\\{value}")))
}

fn class_atom(
    value: char,
    chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<String, ConfigError> {
    if value == '\\' {
        return escaped_atom(chars, true);
    }
    Ok(match value {
        '-' => r"\x2d".into(),
        '&' => r"\x26".into(),
        '~' => r"\x7e".into(),
        '[' => r"\x5b".into(),
        '^' => r"\x5e".into(),
        _ => value.to_string(),
    })
}

fn character_class(
    chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<String, ConfigError> {
    let mut output = String::from("[");
    if chars.peek() == Some(&'^') {
        chars.next();
        output.push('^');
    }
    while let Some(value) = chars.next() {
        if value == ']' {
            output.push(']');
            break;
        }
        output.push_str(&class_atom(value, chars)?);
        if chars.peek() == Some(&'-') {
            chars.next();
            if chars.peek() == Some(&']') {
                output.push_str(r"\x2d");
            } else if let Some(end) = chars.next() {
                output.push('-');
                output.push_str(&class_atom(end, chars)?);
            }
        }
        if output.len() > 1024 * 1024 {
            return Err(invalid("translated pattern exceeds 1 MiB"));
        }
    }
    Ok(output)
}

fn repetition_weight(ast: &regex_syntax::ast::Ast) -> Result<u32, ConfigError> {
    use regex_syntax::ast::{Ast, RepetitionKind, RepetitionRange};
    let weight = match ast {
        Ast::Group(group) => repetition_weight(&group.ast)?,
        Ast::Concat(concat) => concat
            .asts
            .iter()
            .map(repetition_weight)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(1),
        Ast::Alternation(alternation) => alternation
            .asts
            .iter()
            .map(repetition_weight)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(1),
        Ast::Repetition(repetition) => {
            let inner = repetition_weight(&repetition.ast)?;
            let count = match repetition.op.kind {
                RepetitionKind::Range(
                    RepetitionRange::Exactly(count)
                    | RepetitionRange::AtLeast(count)
                    | RepetitionRange::Bounded(_, count),
                ) => count,
                _ => 1,
            };
            inner.saturating_mul(count.max(1))
        }
        _ => 1,
    };
    if weight > 1000 {
        return Err(invalid("pinned RE2JS repetition count exceeds 1000"));
    }
    Ok(weight)
}

#[derive(Debug, Clone)]
pub struct FrontMatterPattern(Arc<regex::Regex>);

impl FrontMatterPattern {
    pub fn new(source: &str) -> Result<Self, ConfigError> {
        JsPattern::compile(
            source,
            PatternFlags {
                unicode: true,
                ignore_case: false,
            },
            "frontMatter",
        )?;
        let mut translated = String::new();
        let mut chars = source.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '[' {
                translated.push_str(&character_class(&mut chars)?);
            } else if ch != '\\' {
                translated.push(ch);
            } else {
                translated.push_str(&escaped_atom(&mut chars, false)?);
            }
            if translated.len() > 1024 * 1024 {
                return Err(invalid("translated pattern exceeds 1 MiB"));
            }
        }
        let ast = regex_syntax::ast::parse::ParserBuilder::new()
            .nest_limit(128)
            .build()
            .parse(&translated)
            .map_err(|error| {
                invalid(format!(
                    "unsupported or invalid RE2-compatible pattern: {error}"
                ))
            })?;
        repetition_weight(&ast)?;
        let regex = regex::RegexBuilder::new(&translated)
            .size_limit(1024 * 1024)
            .build()
            .map_err(|error| {
                ConfigError::invalid(
                    "frontMatter",
                    format!("unsupported or invalid RE2-compatible pattern: {error}"),
                )
            })?;
        Ok(Self(Arc::new(regex)))
    }

    pub fn prefix_end(&self, source: &str) -> Result<usize, ProcessingError> {
        let bom = usize::from(source.starts_with('\u{feff}')) * 3;
        let mut text = String::new();
        let mut offsets = vec![bom];
        let mut chars = source[bom..].char_indices().peekable();
        while let Some((index, ch)) = chars.next() {
            let mut end = bom + index + ch.len_utf8();
            if ch == '\r' {
                if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                    chars.next();
                    end += 1;
                }
                text.push('\n');
                offsets.push(end);
            } else {
                text.push(ch);
                offsets.extend(bom + index + 1..=end);
            }
        }
        Ok(self
            .0
            .find(&text)
            .filter(|matched| matched.start() == 0)
            .map_or(bom, |matched| offsets[matched.end()]))
    }
}

impl ResolvedConfig {
    pub(crate) fn source(&self, text: &str) -> Result<Source, ProcessingError> {
        let mut source = Source::new(text.into());
        if let Some(pattern) = &self.front_matter {
            source.set_front_matter_end(pattern.prefix_end(text)?);
            source.set_front_matter_pattern(pattern.clone());
        }
        Ok(source)
    }
}
