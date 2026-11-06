//! One pending provider interaction per Worker. The hook connection owns the
//! request; observations mirror it and a typed response resolves it.

use crate::agent::hooks::ProcessIdentity;
use crate::worker::{WorkerState, lock_recover};
use tokio::sync::oneshot;
use uuid::Uuid;
use yolo_agent_control_contract::{
    NativeControlWitness, NativeInteraction, NativeInteractionChoice, NativeInteractionKind,
};

pub(crate) const ALLOW_CHOICE: &str = "allow";
pub(crate) const DENY_CHOICE: &str = "deny";
const ALLOW_LABEL: &str = "Allow";
const DENY_LABEL: &str = "Deny";

pub(crate) struct PendingInteraction {
    pub(crate) interaction: NativeInteraction,
    pub(crate) terminal_id: Uuid,
    pub(crate) runtime_epoch: Uuid,
    pub(crate) agent: ProcessIdentity,
    pub(crate) decision: oneshot::Sender<bool>,
}

pub(crate) fn permission(
    id: Uuid,
    prompt: String,
) -> Result<NativeInteraction, yolo_agent_control_contract::ControlContractError> {
    NativeInteraction::new(
        id,
        NativeInteractionKind::Approval,
        vec![
            NativeInteractionChoice {
                id: ALLOW_CHOICE.into(),
                label: ALLOW_LABEL.into(),
            },
            NativeInteractionChoice {
                id: DENY_CHOICE.into(),
                label: DENY_LABEL.into(),
            },
        ],
        prompt,
    )
}

impl WorkerState {
    /// Replaces any earlier pending request; dropping its sender resolves the
    /// older hook waiter without a decision.
    pub(crate) fn register_interaction(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        agent: ProcessIdentity,
        interaction: NativeInteraction,
    ) -> oneshot::Receiver<bool> {
        let (decision, receiver) = oneshot::channel();
        let previous = lock_recover(&self.pending_interaction).replace(PendingInteraction {
            interaction,
            terminal_id,
            runtime_epoch,
            agent,
            decision,
        });
        drop(previous);
        receiver
    }

    pub(crate) fn clear_interaction(&self, id: Uuid) {
        let mut pending = lock_recover(&self.pending_interaction);
        if pending
            .as_ref()
            .is_some_and(|pending| pending.interaction.id() == id)
        {
            pending.take();
        }
    }

    /// The observation carries the interaction only when the witness still
    /// names the process that registered it.
    pub(crate) fn observed_interaction(
        &self,
        witness: &NativeControlWitness,
    ) -> Option<NativeInteraction> {
        let fields = witness.fields();
        let pending = lock_recover(&self.pending_interaction);
        let pending = pending.as_ref()?;
        (pending.terminal_id == fields.terminal_id
            && pending.runtime_epoch == fields.runtime_epoch
            && pending.agent.pid == fields.process_pid
            && pending.agent.start_time == fields.process_start_time)
            .then(|| pending.interaction.clone())
    }

    /// Resolves exactly the interaction named by the response, and only when
    /// the same witness still owns the pending request.
    pub(crate) fn resolve_interaction(
        &self,
        id: Uuid,
        witness: &NativeControlWitness,
        allow: bool,
    ) -> bool {
        let mut pending = lock_recover(&self.pending_interaction);
        let matches = pending.as_ref().is_some_and(|pending| {
            let fields = witness.fields();
            pending.interaction.id() == id
                && pending.terminal_id == fields.terminal_id
                && pending.runtime_epoch == fields.runtime_epoch
                && pending.agent.pid == fields.process_pid
                && pending.agent.start_time == fields.process_start_time
        });
        if !matches {
            return false;
        }
        pending
            .take()
            .is_some_and(|pending| pending.decision.send(allow).is_ok())
    }
}
