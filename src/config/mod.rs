mod brace;
mod directives;
mod glob;
mod load;
mod picomatch;
mod resolve;
mod rules;
mod select;

pub use directives::DocumentConfig;
pub use load::CliOptions;
pub use resolve::{ConfigResolver, ResolvedConfig, ResolvedFile};
pub use rules::{RuleConfig, RuleState};
pub use select::{InputSelection, select_inputs};

use std::{fmt, path::PathBuf};

pub(crate) const JS_SPACE: &str = r"[\t\n\x0b\x0c\r \u{00a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]";

pub(crate) fn js_space(ch: char) -> bool {
    matches!(ch, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub code: &'static str,
    pub path: Option<PathBuf>,
    pub property: String,
    pub message: String,
}

impl ConfigError {
    pub(crate) fn new(
        code: &'static str,
        property: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: None,
            property: property.into(),
            message: message.into(),
        }
    }

    pub(crate) fn invalid(property: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new("invalid-config", property, message)
    }

    pub(crate) fn at(mut self, path: &std::path::Path) -> Self {
        if self.path.is_none() {
            self.path = Some(path.into());
        }
        self
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{}: ", path.display())?;
        }
        write!(f, "{}: {}: {}", self.code, self.property, self.message)
    }
}

impl std::error::Error for ConfigError {}
