use super::{CommandCompletion, PtyOwnerCommandFailure};
use std::sync::{Arc, OnceLock};
use tokio::sync::watch;

pub(crate) const PUBLICATION_CHANNEL_INVARIANT: &str =
    "PTY owner settlement publication channel remains available";
pub(crate) const OWNER_SETTLEMENT_ABANDONED_DETAIL: &str =
    "PTY lifetime owner stopped before publishing settlement";

pub(crate) type OwnerSettlement = Arc<OwnerSettlementState>;

#[must_use = "dropping the publisher records an abandoned PTY owner"]
pub(crate) struct OwnerSettlementPublisher {
    pub(crate) settlement: OwnerSettlement,
}

pub(crate) struct OwnerSettlementState {
    pub(crate) completion: OnceLock<CommandCompletion>,
    pub(crate) publication: watch::Sender<()>,
}

impl OwnerSettlementState {
    pub(crate) fn new() -> Self {
        let (publication, _) = watch::channel(());
        Self {
            completion: OnceLock::new(),
            publication,
        }
    }

    pub(crate) fn get(&self) -> Option<&CommandCompletion> {
        self.completion.get()
    }

    pub(crate) fn set(&self, completion: CommandCompletion) -> Result<(), CommandCompletion> {
        self.completion.set(completion)?;
        self.publication.send_replace(());
        Ok(())
    }

    pub(crate) async fn wait(&self) -> CommandCompletion {
        let mut publication = self.publication.subscribe();
        loop {
            if let Some(completion) = self.get() {
                return completion.clone();
            }
            publication
                .changed()
                .await
                .expect(PUBLICATION_CHANNEL_INVARIANT);
        }
    }
}

impl OwnerSettlementPublisher {
    pub(crate) fn new(settlement: &OwnerSettlement) -> Self {
        Self {
            settlement: Arc::clone(settlement),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(settlement: &OwnerSettlement) -> Self {
        Self::new(settlement)
    }
}

impl Drop for OwnerSettlementPublisher {
    fn drop(&mut self) {
        let _ = self.settlement.set(Err(PtyOwnerCommandFailure::Owner(
            OWNER_SETTLEMENT_ABANDONED_DETAIL.to_owned(),
        )));
    }
}

pub(crate) fn new_owner_settlement() -> (OwnerSettlement, OwnerSettlementPublisher) {
    let settlement = Arc::new(OwnerSettlementState::new());
    let publisher = OwnerSettlementPublisher::new(&settlement);
    (settlement, publisher)
}

#[cfg(test)]
pub(crate) fn new_owner_settlement_for_test() -> OwnerSettlement {
    Arc::new(OwnerSettlementState::new())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_settlement_tests.rs"]
mod tests;
