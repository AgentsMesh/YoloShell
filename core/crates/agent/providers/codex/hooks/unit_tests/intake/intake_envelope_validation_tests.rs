use super::intake_test_support::IntakeFixture;
use crate::agent::hooks::{EventKind, HookProviderKind, ProviderVersion};
use std::path::PathBuf;

const UNSUPPORTED_PROVIDER_VERSION: ProviderVersion = ProviderVersion::new(0, 150, 0);
const INVALID_METADATA_PATH: &str = "relative/intake";

#[test]
fn intake_retains_bounded_events_that_reducer_will_reject() {
    let fixture = IntakeFixture::new();
    for event in [
        provider_mismatch(&fixture),
        version_mismatch(&fixture),
        unsupported_notification(&fixture),
        missing_subagent_identity(&fixture),
    ] {
        fixture.intake().persist_occurrence(event).unwrap();
    }

    assert_eq!(fixture.intake().scan().unwrap().occurrences.len(), 4);
}

#[test]
fn intake_rejects_events_without_a_bounded_reducer_interpretation() {
    let fixture = IntakeFixture::new();
    let mut unsupported = fixture.envelope();
    unsupported.event = EventKind::StopFailure;
    assert!(fixture.intake().persist_occurrence(unsupported).is_err());

    let mut invalid_metadata = provider_mismatch(&fixture);
    invalid_metadata.metadata.cwd = Some(PathBuf::from(INVALID_METADATA_PATH));
    assert!(
        fixture
            .intake()
            .persist_occurrence(invalid_metadata)
            .is_err()
    );
}

fn provider_mismatch(fixture: &IntakeFixture) -> crate::agent::hooks::HookEnvelope {
    let mut event = fixture.envelope();
    event.provider = HookProviderKind::Claude;
    event
}

fn version_mismatch(fixture: &IntakeFixture) -> crate::agent::hooks::HookEnvelope {
    let mut event = fixture.envelope();
    event.provider_version = UNSUPPORTED_PROVIDER_VERSION;
    event
}

fn unsupported_notification(fixture: &IntakeFixture) -> crate::agent::hooks::HookEnvelope {
    let mut event = fixture.envelope();
    event.event = EventKind::Notification {
        kind: crate::agent::hooks::NotificationKind::AgentNeedsInput,
    };
    event
}

fn missing_subagent_identity(fixture: &IntakeFixture) -> crate::agent::hooks::HookEnvelope {
    let mut event = fixture.envelope();
    event.event = EventKind::SubagentStart;
    event
}
