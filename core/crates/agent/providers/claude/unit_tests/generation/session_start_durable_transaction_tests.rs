use super::super::{GenerationApplyDisposition, GenerationEvent, SessionStartSource};
use super::support::Fixture;
use crate::claude_hooks::{ClaudeHookCredentials, ClaudeHookState, ClaudeLifecycleEvent};

const EVENT_SEQUENCE: u64 = crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE;

#[test]
fn prepared_outbox_and_generation_cas_reconcile_at_every_crash_boundary() {
    let fixture = Fixture::new();
    let authority = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    let hook_event = hook_event(&event);
    let ClaudeLifecycleEvent::SessionStart { event_id, .. } = hook_event else {
        unreachable!();
    };
    let transcript = transcript_identity(&event);
    let credentials = credentials(&fixture);
    let agent = process(&fixture);
    let outbox = outbox(&fixture);
    crate::claude_hooks::outbox::enqueue_for_recovery(
        &outbox,
        credentials,
        hook_event,
        Some(transcript),
        agent,
    )
    .unwrap();
    let (planned, disposition) = fixture
        .store
        .plan_invocation_transition(&authority, EVENT_SEQUENCE, &event, Some(transcript))
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    let before = crate::sha256_digest::Sha256Digest::capture(&authority).unwrap();
    let after = crate::sha256_digest::Sha256Digest::capture(&planned).unwrap();
    assert_eq!(
        prepare(&outbox, credentials, event_id, agent, before, after),
        crate::claude_hooks::outbox::SessionStartPreparation::Apply
    );
    reconcile(&fixture, &outbox);
    assert_eq!(
        prepare(&outbox, credentials, event_id, agent, before, after),
        crate::claude_hooks::outbox::SessionStartPreparation::Apply
    );

    let (applied, actual, _) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &authority,
            EVENT_SEQUENCE,
            &event,
            Some(transcript),
        )
        .unwrap();
    assert_eq!(actual, GenerationApplyDisposition::Applied);
    assert_eq!(applied, planned);
    reconcile(&fixture, &outbox);
    assert!(
        crate::claude_hooks::outbox::fence_authenticated_session_start(
            &outbox,
            credentials.invocation_id,
            agent,
            Ok,
        )
        .unwrap()
    );
}

fn prepare(
    outbox: &std::path::Path,
    credentials: ClaudeHookCredentials,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
    before: crate::sha256_digest::Sha256Digest,
    after: crate::sha256_digest::Sha256Digest,
) -> crate::claude_hooks::outbox::SessionStartPreparation {
    crate::claude_hooks::outbox::prepare_session_start(
        outbox,
        credentials,
        crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
        event_id,
        agent,
        before,
        after,
    )
    .unwrap()
}

fn reconcile(fixture: &Fixture, outbox: &std::path::Path) {
    crate::claude_hooks::outbox::reconcile_prepared_session_starts(outbox, |invocation_id| {
        fixture
            .store
            .load_invocation_in_namespace(&fixture.lease, invocation_id)?
            .map(|authority| crate::sha256_digest::Sha256Digest::capture(&authority))
            .transpose()
    })
    .unwrap();
}

fn credentials(fixture: &Fixture) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: fixture.begin.terminal_id,
        runtime_epoch: fixture.begin.runtime_epoch,
        invocation_id: fixture.begin.invocation_id,
        hook_token: fixture.begin.hook_token,
        helper_capability: fixture.begin.helper_capability,
    }
}

fn process(fixture: &Fixture) -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: fixture.begin.process.pid,
        start_time: fixture.begin.process.start_time,
    }
}

fn outbox(fixture: &Fixture) -> std::path::PathBuf {
    let settings = fixture
        .binding_path()
        .parent()
        .unwrap()
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    crate::claude_hooks::outbox::path(&settings).unwrap()
}

fn transcript_identity(event: &GenerationEvent) -> crate::paths::identity::ObjectIdentity {
    let GenerationEvent::SessionStart {
        transcript_path, ..
    } = event
    else {
        unreachable!();
    };
    crate::paths::identity::ObjectIdentity::from_file(
        &std::fs::File::open(transcript_path).unwrap(),
    )
    .unwrap()
    .unwrap()
}

fn hook_event(event: &GenerationEvent) -> ClaudeLifecycleEvent {
    let GenerationEvent::SessionStart {
        event_id,
        provider_session_id,
        cwd,
        transcript_path,
        state,
        ..
    } = event
    else {
        unreachable!();
    };
    ClaudeLifecycleEvent::SessionStart {
        event_id: *event_id,
        provider_session_id: provider_session_id.clone(),
        source: crate::claude_hooks::ClaudeSessionStartSource::Fresh,
        cwd: cwd.clone(),
        transcript_path: transcript_path.clone(),
        state: match state {
            crate::agent::AgentState::WaitingForInput => ClaudeHookState::WaitingForInput,
            _ => ClaudeHookState::Running,
        },
    }
}
