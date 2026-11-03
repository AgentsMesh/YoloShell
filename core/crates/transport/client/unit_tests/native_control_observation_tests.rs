use super::*;

#[tokio::test]
async fn native_observation_preserves_target_digest_and_validated_response() {
    let target = target();
    let expected = observation(target, AgentState::Running);
    let digest = expected.witness().fields().binding_digest;
    let server = ReplyServer::start(method::OBSERVE_NATIVE_AGENT, &Some(expected.clone())).await;
    assert_eq!(
        server
            .client
            .native_agent_observation(target, digest)
            .await
            .unwrap(),
        Some(expected)
    );
    let sent: NativeObservationRequest = server.finish().await.decode_payload().unwrap();
    assert_eq!(sent.terminal_id, target.terminal_id);
    assert_eq!(sent.runtime_epoch, target.runtime_epoch);
    assert_eq!(sent.binding_digest, digest);
}

#[tokio::test]
async fn native_observation_rejects_foreign_session_epoch_or_binding_even_in_valid_envelope() {
    let requested = target();
    let digest = observation(requested, AgentState::Running)
        .witness()
        .fields()
        .binding_digest;
    for (returned, state) in [
        (
            RuntimeRef::new(Uuid::new_v4(), requested.runtime_epoch),
            AgentState::Running,
        ),
        (
            RuntimeRef::new(requested.terminal_id, Uuid::new_v4()),
            AgentState::Running,
        ),
        (requested, AgentState::WaitingForInput),
    ] {
        let reply = observation(returned, state);
        let server = ReplyServer::start(method::OBSERVE_NATIVE_AGENT, &Some(reply)).await;
        assert_mismatch(
            server
                .client
                .native_agent_observation(requested, digest)
                .await,
        );
        server.finish().await;
    }
}

#[tokio::test]
async fn native_observation_accepts_absence_without_fabricating_a_source() {
    let server = ReplyServer::start(
        method::OBSERVE_NATIVE_AGENT,
        &Option::<NativeControlObservation>::None,
    )
    .await;
    let digest = NativeControlDigest::capture(&COMMAND).unwrap();
    assert!(
        server
            .client
            .native_agent_observation(target(), digest)
            .await
            .unwrap()
            .is_none()
    );
    server.finish().await;
}
