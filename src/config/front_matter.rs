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

fn escaped_atom(chars: &mut Peekable<impl Iterator<Item = char>>, class: bool) -> String {
    let Some(value) = chars.next() else {
        return "\\".into();
    };
    if value == 'c' && chars.peek().is_some_and(char::is_ascii_alphabetic) {
        return format!("\\x{:02x}", u32::from(chars.next().unwrap()) % 32);
    }
    escape(value, class)
        .map(String::from)
        .unwrap_or_else(|| format!("\\{value}"))
}

fn class_atom(value: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> String {
    if value == '\\' {
        return escaped_atom(chars, true);
    }
    match value {
        '-' => r"\x2d".into(),
        '&' => r"\x26".into(),
        '~' => r"\x7e".into(),
        '[' => r"\x5b".into(),
        '^' => r"\x5e".into(),
        _ => value.to_string(),
    }
}

fn character_class(chars: &mut Peekable<impl Iterator<Item = char>>) -> String {
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
        output.push_str(&class_atom(value, chars));
        if chars.peek() == Some(&'-') {
            chars.next();
            if chars.peek() == Some(&']') {
                output.push_str(r"\x2d");
            } else if let Some(end) = chars.next() {
                output.push('-');
                output.push_str(&class_atom(end, chars));
            }
        }
    }
    output
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
                translated.push_str(&character_class(&mut chars));
                continue;
            }
            if ch != '\\' {
                translated.push(ch);
                continue;
            }
            translated.push_str(&escaped_atom(&mut chars, false));
        }
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
        }
        Ok(source)
    }
}
