use super::ConfigError;
use crate::ProcessingError;
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PatternFlags {
    pub unicode: bool,
    pub ignore_case: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct JsPattern {
    regex: Arc<regress::Regex>,
    flags: PatternFlags,
    property: String,
}

impl JsPattern {
    pub(crate) fn compile(
        source: &str,
        flags: PatternFlags,
        property: &str,
    ) -> Result<Self, ConfigError> {
        if source.encode_utf16().count() > 65_536 {
            return Err(ConfigError::invalid(
                property,
                "regular expression exceeds 65,536 UTF-16 units",
            ));
        }
        let mut depth = 0usize;
        let mut class = false;
        let mut escaped = false;
        for ch in source.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if class {
                if ch == ']' {
                    class = false;
                }
                continue;
            }
            match ch {
                '[' => class = true,
                '(' => {
                    depth += 1;
                    if depth > 64 {
                        return Err(ConfigError::invalid(
                            property,
                            "regular expression exceeds 64 nested groups",
                        ));
                    }
                }
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        let units = if flags.unicode {
            source.chars().map(u32::from).collect()
        } else {
            super::picomatch::regex_units(source)
        };
        let regex = regress::Regex::from_unicode(
            units.into_iter(),
            regress::Flags {
                unicode: flags.unicode,
                icase: flags.ignore_case,
                ..Default::default()
            },
        )
        .map_err(|error| {
            ConfigError::invalid(
                property,
                format!("invalid JavaScript regular expression: {error}"),
            )
        })?;
        Ok(Self {
            regex: Arc::new(regex),
            flags,
            property: property.into(),
        })
    }

    pub(crate) fn is_match(&self, input: &str) -> Result<bool, ProcessingError> {
        let units: Vec<_> = input.encode_utf16().collect();
        let (found, exhausted) = if self.flags.unicode {
            let mut matches = self.regex.find_from_utf16(&units, 0);
            let found = matches.next().is_some();
            (found, matches.execution_limit_exceeded())
        } else {
            let mut matches = self.regex.find_from_ucs2(&units, 0);
            let found = matches.next().is_some();
            (found, matches.execution_limit_exceeded())
        };
        if exhausted {
            Err(ProcessingError::WorkLimit(format!(
                "{} regular expression exceeded its execution budget",
                self.property
            )))
        } else {
            Ok(found)
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct PatternCache(RefCell<BTreeMap<(String, PatternFlags, String), JsPattern>>);

impl PatternCache {
    pub(crate) fn get(
        &self,
        source: &str,
        flags: PatternFlags,
        property: &str,
    ) -> Result<JsPattern, ProcessingError> {
        let key = (source.into(), flags, property.into());
        let mut cache = self.0.borrow_mut();
        if let Some(pattern) = cache.get(&key) {
            return Ok(pattern.clone());
        }
        if cache.len() >= 256
            || cache
                .keys()
                .map(|key| key.0.len())
                .sum::<usize>()
                .saturating_add(source.len())
                > 1024 * 1024
        {
            return Err(ProcessingError::WorkLimit(format!(
                "{property} document pattern cache exceeds its budget"
            )));
        }
        let compiled =
            JsPattern::compile(source, flags, property).map_err(ProcessingError::Configuration)?;
        cache.insert(key, compiled.clone());
        Ok(compiled)
    }
}
