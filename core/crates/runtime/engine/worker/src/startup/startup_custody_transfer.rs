use crate::pty_custody::CustodyHolderIdentity;
use anyhow::{Result, ensure};
use std::sync::Mutex;

pub(crate) const TRANSFER_HOLDER_MISMATCH_DETAIL: &str =
    "startup PTY custody transfer changed its exact Host holder";

pub(crate) struct StartupCustodyTransferFence {
    pub(crate) holder: Mutex<Option<CustodyHolderIdentity>>,
}

impl StartupCustodyTransferFence {
    pub(crate) fn new() -> Self {
        Self {
            holder: Mutex::new(None),
        }
    }

    pub(crate) fn mark_transfer_possible(&self, holder: CustodyHolderIdentity) -> Result<()> {
        let mut current = self
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ensure!(
            current.is_none() || *current == Some(holder),
            TRANSFER_HOLDER_MISMATCH_DETAIL
        );
        *current = Some(holder);
        Ok(())
    }

    pub(crate) fn holder(&self) -> Option<CustodyHolderIdentity> {
        *self
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn transfer_possible(&self) -> bool {
        self.holder().is_some()
    }
}
