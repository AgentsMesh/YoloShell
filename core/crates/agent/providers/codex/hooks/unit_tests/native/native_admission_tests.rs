use super::test_support::{Fixture, INITIAL_ROUTE, request};
use crate::codex_hooks::{CodexAdmissionAuthority, CodexAdmissionRecord};

#[test]
fn native_admission_atomically_embeds_first_event_without_shim_or_snapshot() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let event = callback.envelope(&fixture.identity).unwrap();
    let record = CodexAdmissionRecord::capture_native_with_route_revision(
        fixture.request.clone(),
        INITIAL_ROUTE,
        event.clone(),
    )
    .unwrap();
    assert_eq!(record.first_event(), &event);
    let wire = serde_json::to_value(&record).unwrap();
    assert!(wire.get("shim_executable").is_none());
    assert!(wire["request"].get("shim_executable").is_none());
    let storage = fixture.storage();
    let published = storage.publish(&record).unwrap();
    let admission = CodexAdmissionAuthority::from_publication(storage, published).unwrap();
    let recovered = CodexAdmissionAuthority::recover_artifacts(&fixture.artifacts)
        .unwrap()
        .unwrap();
    assert!(admission.exactly_matches(&recovered));
    assert!(!fixture.artifacts.snapshot_path().exists());
}

#[test]
fn admission_encoding_requires_the_atomic_first_event() {
    let fixture = Fixture::new();
    let record = fixture.record();
    let encoded = serde_json::to_value(&record).unwrap();
    assert_eq!(
        encoded["request"],
        serde_json::to_value(&fixture.request).unwrap()
    );
    assert!(encoded.get("first_event").is_some());
    let mut missing = encoded.clone();
    missing.as_object_mut().unwrap().remove("first_event");
    assert!(serde_json::from_value::<CodexAdmissionRecord>(missing).is_err());
    let decoded: CodexAdmissionRecord = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, record);
}

#[test]
fn native_admission_rejects_wrong_first_event_identity_and_nonstart() {
    let fixture = Fixture::new();
    let event = request(&fixture).envelope(&fixture.identity).unwrap();
    let context = fixture.request.clone();
    for changed in [
        {
            let mut event = event.clone();
            event.identity.hook_token = uuid::Uuid::new_v4();
            event
        },
        {
            let mut event = event.clone();
            event.sequence = crate::agent::hooks::EventSequence::FIRST;
            event
        },
        {
            let mut event = event.clone();
            event.event_id = uuid::Uuid::nil();
            event
        },
        {
            let mut event = event.clone();
            event.metadata.cwd = None;
            event
        },
        super::test_support::prompt(&fixture)
            .envelope(&fixture.identity)
            .unwrap(),
    ] {
        assert!(
            CodexAdmissionRecord::capture_native_with_route_revision(
                context.clone(),
                INITIAL_ROUTE,
                changed
            )
            .is_err()
        );
    }
    assert!(
        CodexAdmissionRecord::capture_native_with_route_revision(context, u64::default(), event)
            .is_err()
    );
}

#[test]
fn native_context_rejects_unpinned_version_and_invalid_paths() {
    let fixture = Fixture::new();
    let mut context = fixture.request.clone();
    context.hook_executable = Default::default();
    assert!(context.validate().is_err());
    let mut context = fixture.request.clone();
    context.identity.generation = crate::agent::hooks::Generation(u64::default());
    assert!(context.validate().is_err());
}
