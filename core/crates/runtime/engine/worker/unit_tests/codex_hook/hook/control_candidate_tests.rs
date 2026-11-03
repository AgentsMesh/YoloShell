use super::support::ControlSource;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::CodexAuthorityStatus;
use crate::agent::hooks::{ApplyDisposition, EventKind, HookMetadata};
use crate::agent::{AgentRecoveryAvailability, AgentState};
use crate::worker::codex_hook::projection::test_support::{
    FOURTH_SEQUENCE, THIRD_SEQUENCE, submit_event,
};
use yolo_agent_control_contract::{
    NativeControlObservation, NativeControlWitness, NativeControlWitnessFields, authority_digest,
    binding_digest,
};

#[test]
fn candidate_callbacks_keep_exact_control_and_recovery_authority_but_never_prompt_readiness() {
    for (event, continued, expected) in [
        (EventKind::Stop, Some(false), AgentState::Active),
        (EventKind::Stop, Some(true), AgentState::Active),
        (
            EventKind::PermissionRequest,
            None,
            AgentState::WaitingForPermission,
        ),
    ] {
        let source = ControlSource::new();
        let before = source.authority();
        let input = source.projection.authority.as_ref().unwrap();
        let state = submit_event(
            input,
            THIRD_SEQUENCE,
            event,
            HookMetadata::default(),
            continued,
        );
        source.persist_current();
        let current = source.authority();
        assert_eq!(current.status, CodexAuthorityStatus::Active);
        assert_eq!(current.binding.state, expected);
        assert_eq!(
            current.binding.recovery_availability,
            AgentRecoveryAvailability::Available
        );
        assert_eq!(current.provider_process(), before.provider_process());
        assert_eq!(
            current.binding.provider_session_id,
            before.binding.provider_session_id
        );
        assert_eq!(current.binding.evidence_path, before.binding.evidence_path);
        current.verify_resume_evidence().unwrap();
        let worker = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let runtime = AuthorityRuntimeIdentity {
            runtime_epoch: input.identity.runtime_epoch,
            worker: AuthorityProcessIdentity {
                pid: worker.pid,
                start_time: worker.start_time,
            },
        };
        assert_eq!(
            current.recoverable_binding(runtime).unwrap(),
            Some(current.binding.clone())
        );
        let replay = state.history.values().next_back().unwrap().clone();
        assert_eq!(
            input
                .runtime
                .ingest(replay, input.identity.process.pid)
                .unwrap()
                .disposition,
            ApplyDisposition::Replay
        );
        source.persist_current();
        assert_eq!(source.authority(), current);
        let digest = source
            .run(|lineage, verify| {
                verify()?;
                Ok(authority_digest(lineage)?)
            })
            .unwrap();
        let witness = NativeControlWitness::new(NativeControlWitnessFields {
            terminal_id: current.binding.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            worker_id: uuid::Uuid::new_v4(),
            boot_id: crate::process::boot_id(),
            kind: current.binding.kind.clone(),
            process_pid: current.provider_process().pid,
            process_start_time: current.provider_process().start_time,
            binding_digest: binding_digest(&current.binding).unwrap(),
            authority_digest: digest,
        })
        .unwrap();
        assert!(
            !NativeControlObservation::new(current.binding.clone(), witness.clone(), false)
                .unwrap()
                .prompt_input_ready()
        );
        assert!(NativeControlObservation::new(current.binding, witness, true).is_err());
    }
}

#[test]
fn later_provider_facts_resolve_candidates_without_assuming_other_hook_decisions() {
    for (candidate, next, expected) in [
        (EventKind::Stop, EventKind::PreToolUse, AgentState::Running),
        (
            EventKind::Stop,
            EventKind::UserPromptSubmit,
            AgentState::Running,
        ),
        (
            EventKind::PermissionRequest,
            EventKind::PreToolUse,
            AgentState::Running,
        ),
        (
            EventKind::PermissionRequest,
            EventKind::PostToolUse,
            AgentState::Running,
        ),
        (
            EventKind::PermissionRequest,
            EventKind::Stop,
            AgentState::Active,
        ),
    ] {
        let source = ControlSource::new();
        let input = source.projection.authority.as_ref().unwrap();
        let continued = matches!(&candidate, EventKind::Stop).then_some(false);
        let first = if matches!(candidate, EventKind::PermissionRequest) {
            AgentState::WaitingForPermission
        } else {
            AgentState::Active
        };
        submit_event(
            input,
            THIRD_SEQUENCE,
            candidate,
            HookMetadata::default(),
            continued,
        );
        source.persist_current();
        assert_eq!(source.authority().binding.state, first);
        let continued = matches!(&next, EventKind::Stop).then_some(false);
        submit_event(
            input,
            FOURTH_SEQUENCE,
            next,
            HookMetadata::default(),
            continued,
        );
        source.persist_current();
        assert_eq!(source.authority().binding.state, expected);
        source.run(|_, verify| verify()).unwrap();
    }
}
