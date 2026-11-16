use super::constants::SOURCE_CHANGED;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};
use crate::worker::{WorkerState, lock_recover, process, run_process_query};
use anyhow::{Result, anyhow, bail, ensure};
use yolo_agent_control_contract::{NativeControlWitness, NativeTurnEnd, NativeTurnEndKind};

/// Read-only PTY discovery scope; each use re-observes foreground membership.
pub(super) struct PromptInputProof {
    group_id: i32,
    scope: process::PtyProcessScope,
}

impl WorkerState {
    pub(super) async fn observe_prompt_input(&self) -> Option<PromptInputProof> {
        let root = self.pty_root_identity();
        let group_id = self.foreground_process_group().ok()?;
        let scope = run_process_query(move || process::PtyProcessScope::capture(root))
            .await
            .ok()?;
        Some(PromptInputProof { group_id, scope })
    }
}

impl PromptInputProof {
    /// `Ok(None)` means the PTY is idle. `Ok(Some(end))` means the binding is
    /// still a Stop candidate but the provider turn journal proves the turn
    /// ended. Any error means input is not ready.
    pub(super) fn verify(
        &self,
        state: &WorkerState,
        binding: &AgentBinding,
        witness: &NativeControlWitness,
    ) -> Result<Option<NativeTurnEnd>> {
        self.verify_pty(state, witness)?;
        if binding.state == AgentState::WaitingForInput {
            return Ok(None);
        }
        let provider = state
            .provider_runtime
            .get(&binding.kind)
            .ok_or_else(|| anyhow!(SOURCE_CHANGED))?;
        let probe = provider
            .turn_end_probe()
            .ok_or_else(|| anyhow!(SOURCE_CHANGED))?;
        if binding.recovery_availability != AgentRecoveryAvailability::Available
            || !matches!(binding.state, AgentState::Active | AgentState::Running)
        {
            bail!(SOURCE_CHANGED);
        }
        let end = probe
            .read_turn_end(binding)
            .map_err(|_| anyhow!(SOURCE_CHANGED))?
            .ok_or_else(|| anyhow!(SOURCE_CHANGED))?;
        Ok(Some(NativeTurnEnd {
            turn_id: end.turn_id,
            kind: match end.kind {
                yolo_agent_providers::TurnEndKind::Completed => NativeTurnEndKind::Completed,
                yolo_agent_providers::TurnEndKind::Aborted => NativeTurnEndKind::Aborted,
            },
        }))
    }

    fn verify_pty(&self, state: &WorkerState, witness: &NativeControlWitness) -> Result<()> {
        ensure!(
            state.foreground_process_group()? == self.group_id,
            SOURCE_CHANGED
        );
        let fields = witness.fields();
        ensure!(
            self.scope.foreground_contains(
                self.group_id,
                process::ProcessIdentity {
                    pid: fields.process_pid,
                    start_time: fields.process_start_time,
                }
            )?,
            SOURCE_CHANGED
        );
        ensure!(
            lock_recover(&state.terminal).screen().bracketed_paste(),
            SOURCE_CHANGED
        );
        Ok(())
    }
}
