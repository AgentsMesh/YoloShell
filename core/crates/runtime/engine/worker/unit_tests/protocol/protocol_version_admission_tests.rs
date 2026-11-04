use crate::worker::tests::*;
const INCOMPATIBLE_WORKER_PROTOCOL_OFFSET: u16 = 1;
const NEXT_CONTROLLER_GENERATION_OFFSET: u64 = 1;
const GENERATION_REPLICA_COUNT: usize = 4;
const INCOMPATIBLE_AUTH_REQUEST_ID: &str = "incompatible-auth";
const WRONG_KIND_AUTH_REQUEST_ID: &str = "wrong-kind-auth";
const INCOMPATIBLE_CUSTODY_REQUEST_ID: &str = "incompatible-custody";

#[derive(Debug, Eq, PartialEq)]
struct ProtocolAdmissionState {
    generations: [u64; GENERATION_REPLICA_COUNT],
    custody: Option<secondary_custody_authority::ReadyWorkerCustodyIdentity>,
    custody_source_present: bool,
}

fn admission_state(fixture: &Fixture) -> ProtocolAdmissionState {
    let persisted: RuntimeRecord =
        read_json(&fixture.state.paths.runtime_record(fixture.terminal_id())).unwrap();
    ProtocolAdmissionState {
        generations: [
            fixture.state.controller_generation.load(Ordering::Acquire),
            fixture
                .state
                .controller_persisted_generation
                .load(Ordering::Acquire),
            lock_recover(&fixture.state.record).controller_generation,
            persisted.controller_generation,
        ],
        custody: fixture
            .state
            .secondary_custody_authority
            .active_for_test(),
        custody_source_present: lock_recover(&fixture.state.secondary_custody).is_some(),
    }
}

fn auth_request(request_id: &str, fixture: &Fixture) -> RequestEnvelope {
    let original = fixture.state.controller_generation.load(Ordering::Acquire);
    RequestEnvelope::new(
        request_id,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.to_owned(),
            controller_generation: original.saturating_add(NEXT_CONTROLLER_GENERATION_OFFSET),
        })
        .unwrap(),
    )
}

async fn assert_protocol_rejected(fixture: &Fixture, request: RequestEnvelope) {
    let before = admission_state(fixture);
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });

    write_frame(&mut client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert!(!response.ok);
    assert_eq!(response.kind, crate::protocol::RESPONSE_KIND);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    task.await.unwrap().unwrap();
    assert_eq!(admission_state(fixture), before);
}

#[tokio::test]
async fn incompatible_auth_cannot_advance_controller_generation() {
    let fixture = Fixture::new();
    let mut request = auth_request(INCOMPATIBLE_AUTH_REQUEST_ID, &fixture);
    request.version =
        crate::model::PROTOCOL_VERSION.saturating_sub(INCOMPATIBLE_WORKER_PROTOCOL_OFFSET);
    assert_protocol_rejected(&fixture, request).await;
}

#[tokio::test]
async fn wrong_envelope_kind_cannot_advance_controller_generation() {
    let fixture = Fixture::new();
    let mut request = auth_request(WRONG_KIND_AUTH_REQUEST_ID, &fixture);
    request.kind = crate::protocol::RESPONSE_KIND.to_owned();
    assert_protocol_rejected(&fixture, request).await;
}

#[tokio::test]
async fn incompatible_secondary_custody_opening_cannot_enter_adoption() {
    let fixture = Fixture::new();
    let record = lock_recover(&fixture.state.record).clone();
    let custody = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        crate::pty_custody::CustodyHolderIdentity::capture().unwrap(),
        record.auth_token.clone(),
        record
            .controller_generation
            .saturating_add(NEXT_CONTROLLER_GENERATION_OFFSET),
    );
    let mut request = RequestEnvelope::new(
        INCOMPATIBLE_CUSTODY_REQUEST_ID,
        crate::pty_custody::METHOD,
        to_value(&custody).unwrap(),
    );
    request.version =
        crate::model::PROTOCOL_VERSION.saturating_sub(INCOMPATIBLE_WORKER_PROTOCOL_OFFSET);
    assert_protocol_rejected(&fixture, request).await;
}
