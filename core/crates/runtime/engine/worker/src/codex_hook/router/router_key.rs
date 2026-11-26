use crate::agent::hooks::HookIdentity;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::worker::codex_hook) struct CodexRouteKey {
    runtime_epoch: Uuid,
    invocation_id: Uuid,
}

impl CodexRouteKey {
    pub(super) fn from_identity(identity: &HookIdentity) -> Self {
        Self {
            runtime_epoch: identity.runtime_epoch,
            invocation_id: identity.invocation_id,
        }
    }

    pub(in crate::worker::codex_hook) fn runtime_epoch(self) -> Uuid {
        self.runtime_epoch
    }

    pub(in crate::worker::codex_hook) fn invocation_id(self) -> Uuid {
        self.invocation_id
    }
}
