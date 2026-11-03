use crate::support::*;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, OutputCursor};

#[tokio::test]
async fn fragmented_control_calls_preserve_public_correlation_and_disconnect() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    let call = Call::new(
        fixture.hub,
        Command::List {
            include_archived: false,
            after: None,
        },
    )
    .unwrap();
    browser.call(&call).await;
    let answer: Answer = serde_json::from_value(browser.json().await).unwrap();
    assert_eq!(
        answer.reply(&call).unwrap(),
        Reply::Terminals {
            terminals: Vec::new(),
            next: None
        }
    );
    drop(browser);
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn terminal_binary_blobs_are_forwarded_without_utf8_conversion() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: fixture.hub,
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let request = AttachRequest::new(
        target,
        Some(OutputCursor {
            target,
            after_sequence: 0,
        }),
    )
    .unwrap();
    let call = Call::new(fixture.hub, Command::Attach { request }).unwrap();
    browser.call(&call).await;
    let metadata = browser.json().await;
    assert_eq!(
        metadata["result"]["stream"],
        yolo_hub_terminal_wire::STREAM_ENCODING
    );
    assert_eq!(
        metadata["result"]["attachment"]["chunks"][0]["byte_length"],
        TERMINAL_BYTES.len()
    );
    let header = browser.bytes(5).await;
    assert_eq!(header[0], 1);
    assert_eq!(
        u32::from_be_bytes(header[1..].try_into().unwrap()) as usize,
        TERMINAL_BYTES.len()
    );
    assert_eq!(browser.bytes(TERMINAL_BYTES.len()).await, TERMINAL_BYTES);
    drop(browser);
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn dropping_one_browser_preserves_siblings_and_recovers_capacity() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (first, _) = Browser::login(fixture.address).await;
    let (mut second, _) = Browser::login(fixture.address).await;
    assert!(Browser::open(fixture.address, Some(ORIGIN)).await.is_err());
    drop(first);
    fixture.wait_closed(1).await;
    let (third, _) = Browser::login(fixture.address).await;
    let call = Call::new(
        fixture.hub,
        Command::List {
            include_archived: false,
            after: None,
        },
    )
    .unwrap();
    second.call(&call).await;
    let answer: Answer = serde_json::from_value(second.json().await).unwrap();
    assert!(matches!(
        answer.reply(&call).unwrap(),
        Reply::Terminals { .. }
    ));
    drop((second, third));
    fixture.wait_closed(3).await;
    fixture.shutdown().await;
}
