use super::{Driver, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_runtime::model::TerminalView;

const TITLE: &str = "Reviewing terminal presentation";
const NOTIFICATION_TITLE: &str = "Review complete";
const BODY: &str = "Waiting for your decision";
const TITLE_COMMAND: &[u8] = b"printf '\\033]2;Reviewing terminal presentation\\007'; read -r yolo_title_ack; printf '\\033]777;notify;Review complete;Waiting for your decision\\007'; read -r yolo_notification_ack\r";
const ACKNOWLEDGE: &[u8] = b"\r";

pub async fn title_and_notification(driver: &mut Driver, host: &Harness, original: &TerminalView) {
    driver.send(TITLE_COMMAND).await;
    driver
        .wait(|text| text.lines().last().is_some_and(|line| line.contains(TITLE)))
        .await;
    let before = host
        .client()
        .get_session(original.manifest.terminal_id)
        .await
        .unwrap();
    assert_eq!(
        runtime_ref_from_view(&before),
        runtime_ref_from_view(original)
    );
    assert_eq!(
        before
            .runtime
            .as_ref()
            .unwrap()
            .presentation
            .title
            .as_deref(),
        Some(TITLE)
    );
    driver.send(ACKNOWLEDGE).await;
    driver
        .wait(|text| text.lines().last().is_some_and(|line| line.contains(BODY)))
        .await;
    let current = tokio::time::timeout_at(
        deadline(),
        host.client().get_session(original.manifest.terminal_id),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        runtime_ref_from_view(&current),
        runtime_ref_from_view(original)
    );
    let presentation = &current.runtime.as_ref().unwrap().presentation;
    assert!(
        presentation.notification_sequence
            > before
                .runtime
                .as_ref()
                .unwrap()
                .presentation
                .notification_sequence
    );
    let notification = presentation.notification.as_ref().unwrap();
    assert_eq!(notification.title, NOTIFICATION_TITLE);
    assert_eq!(notification.body, BODY);
    driver.send(ACKNOWLEDGE).await;
}
