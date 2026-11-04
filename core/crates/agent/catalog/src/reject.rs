use std::fmt;

/// A request the client refuses before or instead of a transport failure.
/// Command dispatch maps it to the rejected exit code, not a generic error.
#[derive(Debug)]
pub struct Reject(String);

impl fmt::Display for Reject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Reject {}

pub fn rejected(message: impl Into<String>) -> anyhow::Error {
    Reject(message.into()).into()
}
