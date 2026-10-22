mod compile;
mod safety;
#[cfg(test)]
mod tests;

use super::ConfigError;
use regress::Regex;

pub(crate) struct Pattern {
    regex: Regex,
    original: String,
    pub(crate) negated: bool,
}

impl Pattern {
    pub(crate) fn new(pattern: &str, posix: bool) -> Result<Self, ConfigError> {
        let compiled = compile::compile(pattern, posix, 0)?;
        let source = if compiled.negated {
            format!("^(?!^(?:{})$).*$", compiled.output)
        } else {
            format!("^(?:{})$", compiled.output)
        };
        // Picomatch substitutes a never-matching regex when its generated source is invalid.
        let regex =
            Regex::from_unicode(regex_units(&source).into_iter(), regress::Flags::default())
                .unwrap_or_else(|_| Regex::new("$^").expect("constant dead regex"));
        Ok(Self {
            regex,
            original: pattern.into(),
            negated: compiled.negated || compiled.negative_extglob,
        })
    }

    pub(crate) fn regex_matches(&self, path: &str) -> Result<bool, ConfigError> {
        let input: Vec<u16> = path.encode_utf16().collect();
        let mut matches = self.regex.find_from_ucs2(&input, 0);
        let found = matches.next().is_some();
        if matches.execution_limit_exceeded() {
            return Err(ConfigError::new(
                "glob-work-limit",
                &self.original,
                "glob regex exceeded its safe execution budget",
            ));
        }
        Ok(found)
    }

    pub(crate) fn filter_matches(&self, path: &str) -> Result<bool, ConfigError> {
        if path.is_empty() {
            return Ok(false);
        }
        if path == self.original {
            return Ok(true);
        }
        self.regex_matches(path)
    }
}

fn regex_units(source: &str) -> Vec<u32> {
    let input: Vec<_> = source.encode_utf16().collect();
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] == u16::from(b'\\') {
            if input.get(index + 1) == Some(&u16::from(b'\\')) {
                output.extend([u32::from(b'\\'), u32::from(b'\\')]);
                index += 2;
                continue;
            }
            if input.get(index + 1) == Some(&u16::from(b'u')) && index + 6 <= input.len() {
                let hex: String = input[index + 2..index + 6]
                    .iter()
                    .filter_map(|&unit| char::from_u32(u32::from(unit)))
                    .collect();
                if let Ok(unit) = u16::from_str_radix(&hex, 16)
                    && (0xd800..=0xdfff).contains(&unit)
                {
                    output.push(u32::from(unit));
                    index += 6;
                    continue;
                }
            }
        }
        output.push(u32::from(input[index]));
        index += 1;
    }
    output
}
