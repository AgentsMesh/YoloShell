#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("durable filesystem identity is unsupported: {0}")]
    Unsupported(&'static str),
    #[error("invalid durable filesystem identity: {0}")]
    Invalid(&'static str),
    #[error("durable filesystem object identity changed")]
    Changed,
    #[error("{operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    pub(crate) fn system(operation: &'static str) -> Self {
        Self::Io {
            operation,
            source: std::io::Error::last_os_error(),
        }
    }
}
