use super::*;
use crate::host::{ListSessionsRequest, ListSessionsResult, SESSION_MANIFEST_SCHEMA_VERSION};
use crate::model::{RecoveryState, TerminalManifest, TerminalRuntimeStatus, TerminalView};
use crate::protocol::{RequestEnvelope, method, to_value};
use crate::worker::TerminalRef;
use serde::Serialize;
use std::time::Duration;

const OBSERVATION_TIMEOUT: Duration = Duration::from_millis(100);
const OBSERVATION_REQUEST_ID: &str = "custody-observation-test";
const OBSERVATION_SESSION_NAME: &str = "custody-observation";
const NO_CUSTODY_CONNECTION: &str = "catalog observation initiated custody adoption";

#[tokio::test]
async fn get_session_does_not_initiate_custody_adoption() {
    let mut fixture = observation_fixture();
    let terminal_id = fixture.record.terminal_id;
    let (listener, _export, _pending, _custody_path, record) = fixture.take_parts();

    let response = fixture
        .host
        .dispatch(&request(method::GET_SESSION, &TerminalRef { terminal_id }))
        .await;

    assert!(response.ok);
    let view: TerminalView = response.decode_payload().unwrap();
    assert_eq!(view.runtime.unwrap().status, TerminalRuntimeStatus::Unreachable);
    assert_no_adoption(&fixture.host, &record, &listener).await;
}

#[tokio::test]
async fn list_sessions_does_not_initiate_custody_adoption() {
    let mut fixture = observation_fixture();
    let (listener, _export, _pending, _custody_path, record) = fixture.take_parts();

    let response = fixture
        .host
        .dispatch(&request(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: false,
                require_complete: true,
            },
        ))
        .await;

    assert!(response.ok);
    let result: ListSessionsResult = response.decode_payload().unwrap();
    assert!(result.complete);
    assert_eq!(result.sessions.len(), 1);
    assert_eq!(
        result.sessions[0].runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Unreachable
    );
    assert_no_adoption(&fixture.host, &record, &listener).await;
}

fn observation_fixture() -> support::MacosHandshakeFixture {
    let fixture = support::MacosHandshakeFixture::new(None);
    let manifest = TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: fixture.record.terminal_id,
        name: OBSERVATION_SESSION_NAME.to_owned(),
        command: fixture.record.command.clone(),
        args: fixture.record.args.clone(),
        cwd: fixture.record.cwd.clone(),
        cols: fixture.record.cols,
        rows: fixture.record.rows,
        created_at_ms: fixture.record.started_at_ms,
        updated_at_ms: fixture.record.started_at_ms,
        archived: false,
        recovery_state: RecoveryState::Created,
    };
    fixture
        .host
        .catalog
        .lock()
        .unwrap()
        .insert(&manifest)
        .unwrap();
    fixture
}

async fn assert_no_adoption(host: &Host, record: &RuntimeRecord, listener: &UnixListener) {
    assert!(
        tokio::time::timeout(OBSERVATION_TIMEOUT, listener.accept())
            .await
            .is_err(),
        "{NO_CUSTODY_CONNECTION}"
    );
    assert!(!host.rpc_custody_ready(record));
    assert!(
        host.secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(record))
            .is_none()
    );
    assert!(!host.request_drain.is_draining());
}

fn request<T: Serialize>(method_name: &str, payload: &T) -> RequestEnvelope {
    RequestEnvelope::new(
        OBSERVATION_REQUEST_ID,
        method_name,
        to_value(payload).unwrap(),
    )
}
