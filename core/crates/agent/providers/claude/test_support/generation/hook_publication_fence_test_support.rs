use super::super::{HookPublicationAdmission, HookPublicationFence};
use super::observed_unmanaged_publication_test_support::{ABSOLUTE_CLAUDE, observed, scanned};
use super::support::Fixture;
use anyhow::{Result, ensure};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const FENCE_REJECTED: &str = "injected live hook publication fence rejection";

pub(super) fn published_fixture() -> (Fixture, super::super::ClaudeGenerationAuthority) {
    let fixture = Fixture::new();
    let initial = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let event = fixture.start(
        &fixture.begin.provider_session_id,
        super::super::SessionStartSource::Fresh,
    );
    let identity =
        super::observed_unmanaged_publication_test_support::event_identity(&fixture, &event);
    let (running, _, _) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &initial,
            next_sequence(&initial),
            &event,
            Some(identity),
        )
        .unwrap();
    let process = observed(
        &running,
        ABSOLUTE_CLAUDE,
        vec![ABSOLUTE_CLAUDE.to_owned()],
        running.transcript_path.clone().unwrap(),
    );
    fixture
        .lease
        .write_json(&fixture.binding_path(), &scanned(&running, &process))
        .unwrap();
    let admission = HookPublicationAdmission::new(&running, process, AcceptingFence).unwrap();
    let running = publish(&fixture, &running, false, &admission).unwrap().0;
    (fixture, running)
}

pub(super) fn admission(
    authority: &super::super::ClaudeGenerationAuthority,
    gate: Arc<AtomicBool>,
) -> HookPublicationAdmission {
    HookPublicationAdmission::new(
        authority,
        observed(
            authority,
            ABSOLUTE_CLAUDE,
            vec![ABSOLUTE_CLAUDE.to_owned()],
            authority.transcript_path.clone().unwrap(),
        ),
        GateFence(gate),
    )
    .unwrap()
}

pub(super) fn end_admission(
    authority: &super::super::ClaudeGenerationAuthority,
    transcript: std::path::PathBuf,
    transcript_identity: crate::paths::identity::ObjectIdentity,
    gate: Arc<AtomicBool>,
) -> HookPublicationAdmission {
    HookPublicationAdmission::for_session_end(
        authority,
        observed(
            authority,
            ABSOLUTE_CLAUDE,
            vec![ABSOLUTE_CLAUDE.to_owned()],
            transcript.clone(),
        ),
        transcript,
        transcript_identity,
        GateFence(gate),
    )
    .unwrap()
}

pub(super) fn publish(
    fixture: &Fixture,
    authority: &super::super::ClaudeGenerationAuthority,
    force: bool,
    admission: &HookPublicationAdmission,
) -> Result<(
    super::super::ClaudeGenerationAuthority,
    crate::agent::AgentBinding,
)> {
    fixture.store.publish_hook_binding_state_in_namespace(
        &fixture.lease,
        &fixture.binding_path(),
        fixture.authentication(),
        authority,
        force,
        admission,
        |_| Ok(()),
    )
}

pub(super) fn next_sequence(authority: &super::super::ClaudeGenerationAuthority) -> u64 {
    authority.last_event_sequence.checked_add(1).unwrap()
}

struct AcceptingFence;

impl HookPublicationFence for AcceptingFence {
    fn verify(&self, _: &super::super::ClaudeGenerationAuthority) -> Result<()> {
        Ok(())
    }
}

struct GateFence(Arc<AtomicBool>);

impl HookPublicationFence for GateFence {
    fn verify(&self, _: &super::super::ClaudeGenerationAuthority) -> Result<()> {
        ensure!(self.0.load(Ordering::Acquire), FENCE_REJECTED);
        Ok(())
    }
}
