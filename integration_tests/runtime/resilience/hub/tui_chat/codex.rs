use super::{acceptance, constants::*, support};
use crate::hub::{
    support::{RunningHub, SOCKET_TEST_ROOT},
    tui_shell_first::driver::Driver,
};
use yolo_hub_agent_protocol::{
    Activity, Capability, Freshness, Lifecycle, ObservationSource, Reachability, RecoveryStatus,
};

#[tokio::test(flavor = "multi_thread")]
async fn codex_stop_keeps_pty_usable_without_granting_another_chat_prompt() {
    let host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let directory = root.path().canonicalize().unwrap();
    let hub = RunningHub::start(&host, &directory.join(STATE_DIRECTORY)).await;
    let marker = directory.join(MARKER_FILE);
    let target = support::launch(
        &host,
        &hub,
        &marker,
        crate::support::codex_install_spec(),
    )
    .await;
    let initial = support::current(&hub, target).await;
    assert_eq!(initial.fields().activity, Activity::WaitingForInput);
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
    driver.send(PREFIX).await;
    driver.send(CHAT_KEY).await;
    driver.wait(|text| text.contains(APPEND_HINT)).await;
    driver.send(CHAT_APPEND.as_bytes()).await;
    driver.send(ENTER).await;
    let first = format!("{NATIVE_DRAFT}{CHAT_APPEND}");
    acceptance::submissions(&marker, std::slice::from_ref(&first)).await;
    let stopped = support::observed(&hub, target, |agent| {
        agent.cursor() != initial.cursor()
            && agent.fields().activity == Activity::Unknown
            && agent.fields().lifecycle == (Lifecycle::Running {})
            && agent.fields().freshness == Freshness::Current
            && agent.fields().source == ObservationSource::ProviderHook
            && agent.fields().recovery == RecoveryStatus::Available
            && matches!(
                agent.fields().capabilities.submit_prompt,
                Capability::Unavailable { .. }
            )
    })
    .await;
    assert_eq!(stopped.fields().lifecycle, Lifecycle::Running {});
    assert_eq!(stopped.fields().freshness, Freshness::Current);
    assert_eq!(stopped.fields().reachability, Reachability::Reachable);
    assert_eq!(stopped.fields().source, ObservationSource::ProviderHook);
    assert_eq!(stopped.fields().recovery, RecoveryStatus::Available);
    assert_eq!(
        stopped.fields().capabilities.interrupt,
        Capability::Available {}
    );
    driver.wait(|text| text.contains(ANSWER)).await;
    let submitted = std::fs::read(&marker).unwrap();
    driver.send(BLOCKED_CHAT.as_bytes()).await;
    driver.send(ENTER).await;
    driver
        .wait(|text| text.contains(CANNOT_SEND_HINT) || text.contains(REJECTED_HINT))
        .await;
    assert_eq!(std::fs::read(&marker).unwrap(), submitted);
    driver.send(ESCAPE).await;
    driver.wait(|text| text.contains(READY)).await;
    driver.send(MANUAL_PROMPT.as_bytes()).await;
    driver.send(ENTER).await;
    acceptance::submissions(&marker, &[first, MANUAL_PROMPT.into()]).await;
    let after_manual = support::observed(&hub, target, |agent| {
        agent.cursor() != stopped.cursor()
            && agent.fields().activity == Activity::Unknown
            && agent.fields().lifecycle == (Lifecycle::Running {})
            && agent.fields().freshness == Freshness::Current
            && agent.fields().recovery == RecoveryStatus::Available
    })
    .await;
    assert!(matches!(
        after_manual.fields().capabilities.submit_prompt,
        Capability::Unavailable { .. }
    ));
    driver.send(DETACH_KEYS).await;
    driver.returned().await;
    acceptance::same_processes(&host, target, &original).await;
    driver.close().await;
    hub.stop().await;
}
