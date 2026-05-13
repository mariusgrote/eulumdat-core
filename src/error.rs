use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum EulumdatError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Parse(ParseContext),
    Validation(String),
    DistributionShape(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseContext {
    pub field: String,
    pub line_number: usize,
    pub byte_offset: usize,
    pub raw_line: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

impl Display for EulumdatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Utf8(error) => write!(f, "invalid UTF-8 input: {error}"),
            Self::Parse(context) => write!(f, "{context}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::DistributionShape(message) => write!(f, "distribution shape error: {message}"),
        }
    }
}

impl Display for ParseContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at line {}, byte {} ({}): {}",
            self.line_number, self.byte_offset, self.field, self.reason
        )?;
        if let Some(raw_line) = &self.raw_line {
            write!(f, " [{raw_line}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for EulumdatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Parse(_) | Self::Validation(_) | Self::DistributionShape(_) => None,
        }
    }
}

impl From<std::io::Error> for EulumdatError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::str::Utf8Error> for EulumdatError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::Utf8(value)
    }
}
