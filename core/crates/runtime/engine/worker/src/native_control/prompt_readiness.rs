use super::constants::SOURCE_CHANGED;
use crate::agent::{AgentBinding, AgentState};
use crate::worker::{WorkerState, lock_recover, process, run_process_query};
use anyhow::{Result, ensure};
use yolo_agent_control_contract::{NativeControlWitness, NativeTurnEnd};

pub(super) fn observe(
    state: &WorkerState,
    binding: &AgentBinding,
    stop_turn: Option<&str>,
) -> (bool, Option<NativeTurnEnd>) {
    if binding.state == AgentState::WaitingForInput {
        return (true, None);
    }
    let end = stop_turn
        .and_then(|turn| crate::worker::codex_hook::read_stop_turn_end(state, binding, turn));
    (end.is_some(), end)
}

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
    pub(super) fn verify_pty(
        &self,
        state: &WorkerState,
        witness: &NativeControlWitness,
    ) -> Result<()> {
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
