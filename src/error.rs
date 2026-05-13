use std::fmt::{Display, Formatter};

#[derive(Debug)]
/// Error type returned by EULUMDAT parsing, validation, and serialization APIs.
pub enum EulumdatError {
    /// An I/O operation failed.
    Io(std::io::Error),
    /// Strict UTF-8 decoding failed.
    Utf8(std::str::Utf8Error),
    /// A field could not be parsed from text.
    Parse(ParseContext),
    /// A parsed or edited model violates EULUMDAT validation rules.
    Validation(String),
    /// The C-plane, gamma-angle, or intensity matrix dimensions are inconsistent.
    DistributionShape(String),
}

#[derive(Debug, Clone, PartialEq)]
/// Location and field information for a parse error.
pub struct ParseContext {
    /// The EULUMDAT field being read when the error occurred.
    pub field: String,
    /// The one-based input line number.
    pub line_number: usize,
    /// The byte offset at the start of the failed field.
    pub byte_offset: usize,
    /// The original line text, when a line was available.
    pub raw_line: Option<String>,
    /// A short machine-readable reason for the parse failure.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
/// Non-fatal validation issue found in a parsed or edited model.
pub struct ValidationWarning {
    /// The field that triggered the warning.
    pub field: String,
    /// Human-readable warning message.
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
