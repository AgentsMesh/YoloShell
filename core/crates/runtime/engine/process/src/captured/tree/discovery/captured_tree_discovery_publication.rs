use super::CapturedProcessTree;
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use anyhow::Result;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum DiscoveryPublicationAudit {
    #[default]
    Required,
    InProgress,
}

impl CapturedProcessTree {
    pub(super) fn reset_discovery_publication_audit(&mut self) {
        self.discovery_publication_audit = DiscoveryPublicationAudit::Required;
    }

    fn begin_dense_publication_audit(&mut self) {
        self.begin_discovery_cleanup_internal();
        self.discovery_publication_audit = DiscoveryPublicationAudit::InProgress;
    }

    pub(crate) fn discovery_publication_ready_internal(&mut self) -> Result<bool> {
        if !self.discovery_is_authoritatively_empty_internal() {
            return Ok(false);
        }
        if !self.live_identities()?.is_empty() {
            self.begin_discovery_cleanup_internal();
            return Ok(false);
        }
        #[cfg(target_os = "macos")]
        if self.events.is_gated() {
            if self.refresh_if_needed_internal()? {
                return Ok(false);
            }
        } else if self.discovery_publication_audit == DiscoveryPublicationAudit::Required {
            self.begin_dense_publication_audit();
            self.run_dense_publication_audit()?;
        }
        Ok(self.discovery_is_authoritatively_empty_internal())
    }

    fn run_dense_publication_audit(&mut self) -> Result<()> {
        for _attempt in 0..EVENT_RECONCILIATION_LIMIT {
            self.refresh()?;
            if self.discovery_is_authoritatively_empty_internal()
                || !self.live_identities()?.is_empty()
                || self.events.is_gated()
            {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/discovery/captured_tree_discovery_publication_tests.rs"]
mod tests;
