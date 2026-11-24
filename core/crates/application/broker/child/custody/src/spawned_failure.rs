use std::fmt;
use std::io;
use yolo_process::is_child_already_reaped;

pub(super) const WAIT_AUTHORITY_LOST_CONTEXT: &str = "child wait authority was lost";
const FAILURE_SEPARATOR: &str = "; ";

#[derive(Debug)]
pub(super) enum OperationFailure {
    Operation {
        operation: &'static str,
        source: io::Error,
    },
    WaitAuthorityLost {
        operation: &'static str,
        source: io::Error,
    },
}

impl OperationFailure {
    pub(super) fn operation(operation: &'static str, source: io::Error) -> Self {
        Self::Operation { operation, source }
    }

    pub(super) fn wait(operation: &'static str, source: io::Error) -> Self {
        if is_child_already_reaped(&source) {
            Self::WaitAuthorityLost { operation, source }
        } else {
            Self::Operation { operation, source }
        }
    }

    pub(super) fn wait_authority_lost(&self) -> bool {
        matches!(self, Self::WaitAuthorityLost { .. })
    }

    fn source(&self) -> &io::Error {
        match self {
            Self::Operation { source, .. } | Self::WaitAuthorityLost { source, .. } => source,
        }
    }
}

impl fmt::Display for OperationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation { operation, source } => write!(formatter, "{operation}: {source}"),
            Self::WaitAuthorityLost { operation, source } => {
                write!(
                    formatter,
                    "{WAIT_AUTHORITY_LOST_CONTEXT}: {operation}: {source}"
                )
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ChildSettlementFailure {
    failures: Vec<OperationFailure>,
}

impl ChildSettlementFailure {
    pub(super) fn one(failure: OperationFailure) -> Self {
        Self {
            failures: vec![failure],
        }
    }

    pub(super) fn from_failures(failures: Vec<OperationFailure>) -> Self {
        debug_assert!(!failures.is_empty());
        Self { failures }
    }

    #[cfg(test)]
    pub(super) fn wait_authority_lost(&self) -> bool {
        self.failures
            .iter()
            .any(OperationFailure::wait_authority_lost)
    }
}

impl fmt::Display for ChildSettlementFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.failures.iter().enumerate() {
            if index > usize::default() {
                formatter.write_str(FAILURE_SEPARATOR)?;
            }
            write!(formatter, "{failure}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ChildSettlementFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failures
            .first()
            .map(|failure| failure.source() as &(dyn std::error::Error + 'static))
    }
}

#[derive(Debug)]
pub(super) struct CombinedCleanupFailure {
    primary: anyhow::Error,
    cleanup: anyhow::Error,
    context: &'static str,
}

impl CombinedCleanupFailure {
    pub(super) fn new(
        primary: anyhow::Error,
        cleanup: anyhow::Error,
        context: &'static str,
    ) -> Self {
        Self {
            primary,
            cleanup,
            context,
        }
    }
}

impl fmt::Display for CombinedCleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:#}; {}: {:#}",
            self.primary, self.context, self.cleanup
        )
    }
}

impl std::error::Error for CombinedCleanupFailure {}
