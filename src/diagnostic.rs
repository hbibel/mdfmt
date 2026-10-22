use std::{fmt, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub rule_id: Option<String>,
    pub span: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingError {
    InvalidRange(Range<usize>),
    EditConflict,
    ProtectedRegion(Range<usize>),
    Parse(String),
    Configuration(crate::config::ConfigError),
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange(span) => write!(f, "invalid source range {span:?}"),
            Self::EditConflict => f.write_str("conflicting edits"),
            Self::ProtectedRegion(span) => {
                write!(f, "edit crosses protected or ineligible source {span:?}")
            }
            Self::Parse(message) => write!(f, "Markdown parsing failed: {message}"),
            Self::Configuration(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ProcessingError {}
