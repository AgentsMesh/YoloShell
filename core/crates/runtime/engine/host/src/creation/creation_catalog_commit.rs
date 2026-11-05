use std::sync::atomic::{AtomicU8, Ordering};

pub(crate) const CATALOG_COMMIT_PENDING: u8 = 0;
pub(crate) const CATALOG_COMMIT_OWNED: u8 = 1;
pub(crate) const CATALOG_COMMIT_CANCELLED: u8 = 2;
pub(crate) const CATALOG_COMMIT_STATE_ERROR: &str = "invalid creation catalog commit state";

#[derive(Clone)]
pub(in crate::host) struct CreationCatalogCommitGate {
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) deadline: HostOperationDeadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::host) enum CreationCatalogTimeoutDisposition {
    Cancelled,
    CommitOwned,
}

impl CreationCatalogCommitGate {
    pub(in crate::host) fn new(deadline: HostOperationDeadline) -> Self {
        Self {
            state: Arc::new(AtomicU8::new(CATALOG_COMMIT_PENDING)),
            deadline,
        }
    }

    pub(in crate::host) fn claim(&self) -> Result<()> {
        require_creation_commit_budget(self.deadline)?;
        match self.state.compare_exchange(
            CATALOG_COMMIT_PENDING,
            CATALOG_COMMIT_OWNED,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(CATALOG_COMMIT_CANCELLED) => {
                Err(anyhow::Error::new(CreationPublicationDeadlineExpired))
            }
            Err(_) => anyhow::bail!(CATALOG_COMMIT_STATE_ERROR),
        }
    }

    pub(in crate::host) fn cancel(&self) -> CreationCatalogTimeoutDisposition {
        match self.state.compare_exchange(
            CATALOG_COMMIT_PENDING,
            CATALOG_COMMIT_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) | Err(CATALOG_COMMIT_CANCELLED) => CreationCatalogTimeoutDisposition::Cancelled,
            Err(CATALOG_COMMIT_OWNED) => CreationCatalogTimeoutDisposition::CommitOwned,
            Err(_) => unreachable!("{CATALOG_COMMIT_STATE_ERROR}"),
        }
    }
}
