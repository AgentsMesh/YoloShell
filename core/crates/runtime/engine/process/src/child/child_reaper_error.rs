use std::fmt;
use std::io;
use std::process::Child;

#[derive(Clone)]
pub(super) struct InitializationFailure {
    pub(super) kind: io::ErrorKind,
    pub(super) detail: String,
}

pub struct DetachedChildHandoffError {
    source: anyhow::Error,
    child: Child,
}

impl DetachedChildHandoffError {
    pub fn into_parts(self) -> (anyhow::Error, Child) {
        (self.source, self.child)
    }

    pub(super) fn new(source: anyhow::Error, child: Child) -> Self {
        Self { source, child }
    }
}

impl fmt::Debug for DetachedChildHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DetachedChildHandoffError")
            .field("source", &self.source)
            .field("child_id", &self.child.id())
            .finish()
    }
}

impl fmt::Display for DetachedChildHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for DetachedChildHandoffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl From<io::Error> for InitializationFailure {
    fn from(error: io::Error) -> Self {
        Self {
            kind: error.kind(),
            detail: error.to_string(),
        }
    }
}
