use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Violation {
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

impl Violation {
    pub fn file(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line: None,
            message: message.into(),
        }
    }

    pub fn line(path: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line: Some(line),
            message: message.into(),
        }
    }
}

impl Display for Violation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "{}:{line}: {}", self.path, self.message),
            None => write!(formatter, "{}: {}", self.path, self.message),
        }
    }
}
