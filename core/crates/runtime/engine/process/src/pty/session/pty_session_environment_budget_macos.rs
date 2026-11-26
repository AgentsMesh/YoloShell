use super::{
    CapabilityProofError, CapabilityProofResult, CapabilityScanBudget,
    ENVIRONMENT_ITEM_BUDGET_DETAIL,
};
use crate::process::constants::MAX_PTY_PROOF_ENVIRONMENT_BYTES;
use anyhow::anyhow;
use std::mem::MaybeUninit;

const ENVIRONMENT_CAPACITY_CHANGED_DETAIL: &str =
    "maximum PTY proof process argument size changed during a scan";

impl CapabilityScanBudget {
    pub(in super::super) fn cached_environment_capacity(&self) -> Option<usize> {
        (!self.environment_buffer.is_empty()).then_some(self.environment_buffer.len())
    }

    pub(in super::super) fn environment_capture_buffer(
        &mut self,
        capacity: usize,
    ) -> CapabilityProofResult<&mut [MaybeUninit<u8>]> {
        Self::validate_environment_item(capacity)?;
        if self.environment_buffer.is_empty() {
            self.environment_buffer = Vec::with_capacity(capacity);
            // SAFETY: uninitialized storage is a valid MaybeUninit value.
            unsafe { self.environment_buffer.set_len(capacity) };
        } else if self.environment_buffer.len() != capacity {
            return Err(CapabilityProofError::invariant(anyhow!(
                ENVIRONMENT_CAPACITY_CHANGED_DETAIL
            )));
        }
        Ok(&mut self.environment_buffer)
    }

    #[cfg(test)]
    pub(in super::super) fn validate_environment_capacity(
        &self,
        count: usize,
    ) -> CapabilityProofResult<()> {
        Self::validate_environment_item(count)
    }

    pub(super) fn validate_environment_item(count: usize) -> CapabilityProofResult<()> {
        if count > MAX_PTY_PROOF_ENVIRONMENT_BYTES {
            return Err(CapabilityProofError::budget(anyhow!(
                ENVIRONMENT_ITEM_BUDGET_DETAIL
            )));
        }
        Ok(())
    }
}
