mod acceptance;
mod codex;
mod constants;
mod provider;
mod support;
#[path = "test_support/submissions.rs"]
mod submissions;
use crate::hub::{
    support::{RunningHub, SOCKET_TEST_ROOT},
    tui_shell_first::driver::Driver,
};
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_chat_reads_native_content_and_appends_once_without_replacing_the_original_pty() {
    let host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let directory = root.path().canonicalize().unwrap();
    let hub = RunningHub::start(&host, &directory.join(STATE_DIRECTORY)).await;
    let marker = directory.join(MARKER_FILE);
    let target = support::launch(
        &host,
        &hub,
        &marker,
        crate::support::claude_install_spec(),
    )
    .await;
    let initial = support::current(&hub, target).await;
    let original = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let driver_host = crate::Harness::start().await;
    let mut driver = Driver::new(&driver_host).await;
    driver
        .launch(
            &hub,
            &directory.join(CLIENT_DIRECTORY),
            &format!("{ATTACH_COMMAND} {}", target.address),
        )
        .await;
    driver
        .wait(|text| {
            text.contains(READY)
                && text
                    .lines()
                    .last()
                    .is_some_and(|line| line.contains(CONTROL))
        })
        .await;
    driver.send(NATIVE_DRAFT.as_bytes()).await;
    driver
        .wait(|text| text.contains(&format!("{DRAFT}{NATIVE_DRAFT}")))
        .await;
    assert!(std::fs::read(&marker).unwrap().is_empty());
    driver.send(PREFIX).await;
    driver.send(CHAT_KEY).await;
    driver.wait(|text| text.contains(APPEND_HINT)).await;
    driver.send(CHAT_APPEND.as_bytes()).await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(DELIVERY_HINT)).await;
    acceptance::submissions(&marker, &[format!("{NATIVE_DRAFT}{CHAT_APPEND}")]).await;
    // Agent's newly appended native transcript must become visible through the public reader.
    driver.wait(|text| text.contains(ANSWER)).await;
    let submitted = std::fs::read(&marker).unwrap();
    driver.send(ESCAPE).await;
    driver.wait(|text| text.contains(READY)).await;
    driver.send(PREFIX).await;
    driver.send(CHAT_KEY).await;
    driver
        .wait(|text| text.contains(APPEND_HINT) && text.contains(ANSWER))
        .await;
    assert_eq!(std::fs::read(&marker).unwrap(), submitted);
    let ready = support::observed(&hub, target, |agent| {
        agent.cursor() != initial.cursor()
            && agent.fields().capabilities.submit_prompt
                == yolo_hub_agent_protocol::Capability::Available {}
    })
    .await;
    assert_eq!(
        ready.fields().activity,
        yolo_hub_agent_protocol::Activity::WaitingForInput
    );
    driver.send(SECOND_CHAT.as_bytes()).await;
    driver.send(ENTER).await;
    acceptance::submissions(
        &marker,
        &[format!("{NATIVE_DRAFT}{CHAT_APPEND}"), SECOND_CHAT.into()],
    )
    .await;
    support::observed(&hub, target, |agent| {
        agent.cursor() != ready.cursor()
            && agent.fields().capabilities.submit_prompt
                == yolo_hub_agent_protocol::Capability::Available {}
    })
    .await;
    driver.send(DETACH_KEYS).await;
    driver.returned().await;
    let after = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let before = original.runtime.as_ref().unwrap();
    let now = after.runtime.as_ref().unwrap();
    assert_eq!(
        (
            before.runtime_epoch,
            before.worker_pid,
            before.worker_start_time,
            before.child_pid,
            before.child_start_time
        ),
        (
            now.runtime_epoch,
            now.worker_pid,
            now.worker_start_time,
            now.child_pid,
            now.child_start_time
        )
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        original.agent.as_ref().unwrap().process_pid
    );
    driver.close().await;
    hub.stop().await;
}
