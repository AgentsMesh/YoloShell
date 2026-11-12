use super::*;
use crate::worker::native_control::NativeRespondRequest;
use crate::worker::native_control::interaction;
use yolo_crypto::Sha256Digest;

#[tokio::test]
async fn respond_resolves_exactly_one_pending_interaction() {
    let source = requests::prepared();
    let fixture = &source.fixture;
    let state = &fixture.state;
    let observed = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    let witness = observed.witness().clone();
    let (generation, writer_capability) = {
        let mut lease_request = fixture.lease_request(WRITER);
        lease_request.ttl_ms = Some(WRITER_TTL_MS);
        let lease = state
            .acquire_lease(fixture.controller(), &lease_request, false)
            .await
            .unwrap();
        (lease.generation, lease.writer_capability)
    };
    let interaction_id = Uuid::new_v4();
    let decision = state.register_interaction(
        fixture.terminal_id(),
        state.runtime_epoch(),
        crate::agent::hooks::ProcessIdentity::new(
            witness.fields().process_pid,
            witness.fields().process_start_time,
        ),
        interaction::permission(interaction_id, "Bash: cargo test".into()).unwrap(),
    );
    let observed = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        observed.interaction().map(|pending| pending.id()),
        Some(interaction_id)
    );
    let request = NativeRespondRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        request_id: Uuid::new_v4(),
        generation,
        writer_capability,
        witness: observed.witness().clone(),
        command_digest: Sha256Digest::capture(&COMMAND).unwrap(),
        interaction: interaction_id,
        choice: "allow".into(),
    };
    let result = state
        .respond_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::Delivered);
    assert_eq!(decision.await.unwrap(), true);
    let after = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    assert!(after.interaction().is_none());

    let replay = state
        .respond_native_agent(fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(replay.outcome, NativeControlOutcome::Delivered);
}

#[tokio::test]
async fn respond_rejects_a_replaced_interaction_or_stale_generation_without_consuming_it() {
    let source = requests::prepared();
    let fixture = &source.fixture;
    let state = &fixture.state;
    let observed = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    let witness = observed.witness().clone();
    let (generation, writer_capability) = {
        let mut lease_request = fixture.lease_request(WRITER);
        lease_request.ttl_ms = Some(WRITER_TTL_MS);
        let lease = state
            .acquire_lease(fixture.controller(), &lease_request, false)
            .await
            .unwrap();
        (lease.generation, lease.writer_capability)
    };
    let interaction_id = Uuid::new_v4();
    let decision = state.register_interaction(
        fixture.terminal_id(),
        state.runtime_epoch(),
        crate::agent::hooks::ProcessIdentity::new(
            witness.fields().process_pid,
            witness.fields().process_start_time,
        ),
        interaction::permission(interaction_id, "Bash: cargo test".into()).unwrap(),
    );
    let observed = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    let request = NativeRespondRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        request_id: Uuid::new_v4(),
        generation,
        writer_capability,
        witness: observed.witness().clone(),
        command_digest: Sha256Digest::capture(&COMMAND).unwrap(),
        interaction: interaction_id,
        choice: "allow".into(),
    };
    let mut replaced = request.clone();
    replaced.request_id = Uuid::new_v4();
    replaced.interaction = Uuid::new_v4();
    replaced.command_digest = Sha256Digest::capture(b"replaced").unwrap();
    assert_eq!(
        state
            .respond_native_agent(fixture.controller(), &replaced)
            .await
            .unwrap()
            .outcome,
        NativeControlOutcome::NotDelivered
    );
    let mut stale = request.clone();
    stale.request_id = Uuid::new_v4();
    stale.generation = generation + 1;
    stale.command_digest = Sha256Digest::capture(b"stale").unwrap();
    assert!(
        state
            .respond_native_agent(fixture.controller(), &stale)
            .await
            .is_err()
    );
    let still = state
        .observe_native_agent(fixture.controller(), &observation(&source))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(still.interaction().map(|pending| pending.id()), Some(interaction_id));
    assert_eq!(
        state
            .respond_native_agent(fixture.controller(), &request)
            .await
            .unwrap()
            .outcome,
        NativeControlOutcome::Delivered
    );
    assert_eq!(decision.await.unwrap(), true);
}
