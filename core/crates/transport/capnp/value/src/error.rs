use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueError {
    Invalid(String),
    Limit,
    Depth,
    DuplicateKey,
    Trailing,
    Schema,
    Utf8,
}

impl fmt::Display for ValueError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Invalid(value) => value,
            Self::Limit => "Cap'n Proto document exceeds its resource budget",
            Self::Depth => "Cap'n Proto document exceeds its nesting budget",
            Self::DuplicateKey => "duplicate structural map key",
            Self::Trailing => "trailing bytes after Cap'n Proto document",
            Self::Schema => "unsupported Cap'n Proto document schema",
            Self::Utf8 => "invalid UTF-8 in Cap'n Proto text",
        })
    }
}
impl std::error::Error for ValueError {}
impl serde::ser::Error for ValueError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Invalid(message.to_string())
    }
}
impl serde::de::Error for ValueError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Invalid(message.to_string())
    }
}
