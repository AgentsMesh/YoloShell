use crate::support::*;
use std::sync::atomic::Ordering;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn origins_and_bearer_reject_before_a_hub_connection_is_opened() {
    let fixture = Fixture::start(LIFETIME, 4).await;
    for origin in [None, Some("https://evil.test"), Some("null")] {
        assert!(Browser::open(fixture.address, origin).await.is_err());
    }
    let mut browser = Browser::open(fixture.address, Some(ORIGIN)).await.unwrap();
    browser.authenticate(&"f".repeat(64)).await;
    browser.closed().await;
    assert_eq!(fixture.accepted.load(Ordering::SeqCst), 0);
    fixture.shutdown().await;
}

#[tokio::test]
async fn public_client_scope_blocks_local_management_even_with_valid_web_token() {
    let fixture = Fixture::start(LIFETIME, 4).await;
    let (mut browser, ready) = Browser::login(fixture.address).await;
    assert_eq!(
        ready[CLIENT_ID_FIELD],
        fixture.identity.client_id.to_string()
    );
    assert_eq!(ready[HELLO_FIELD][HUB_ID_FIELD], fixture.hub.to_string());
    assert!(
        !ready[HELLO_FIELD][METHODS_FIELD]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method == GET_CLIENT_ACCESS_METHOD)
    );
    let call = Call::new(fixture.hub, Command::GetClientAccess {}).unwrap();
    browser.call(&call).await;
    let answer: Answer = serde_json::from_value(browser.json().await).unwrap();
    assert_eq!(
        answer.reply(&call).unwrap(),
        Reply::Error {
            code: PublicError::Unauthorized
        }
    );
    drop(browser);
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn session_expiry_closes_both_websocket_and_scoped_hub_channel() {
    let fixture = Fixture::start(SHORT_LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    browser.closed().await;
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn registry_revocation_closes_browser_without_reconnecting_authority() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    fixture.registry.close();
    browser.closed().await;
    fixture.wait_closed(1).await;
    assert_eq!(fixture.accepted.load(Ordering::SeqCst), 1);
    fixture.shutdown().await;
}
