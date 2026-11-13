use super::fixtures;
use yolo_tui_notification::Notifications;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::{NotificationKind, TerminalNotification, TerminalSummary};

const TITLE: &str = "Task complete";
const BODY: &str = "Tests passed";
const FIRST: u64 = 1;
const NEXT: u64 = 2;
const EXPIRED: Duration = Duration::from_secs(60);

fn update(terminal: &TerminalSummary, sequence: u64) -> TerminalSummary {
    let mut fields = terminal.fields().clone();
    fields.presentation.notification_sequence = sequence;
    fields.presentation.notification = Some(TerminalNotification {
        kind: NotificationKind::Desktop,
        title: TITLE.into(),
        body: BODY.into(),
    });
    TerminalSummary::new(fields).unwrap()
}

#[test]
fn fresh_notifications_ring_once_and_keep_unfocused_attention_after_toast_expires() {
    let terminal = fixtures::session();
    let mut notifications = Notifications::default();
    let mut attention = BTreeSet::new();
    let now = Instant::now();
    notifications.observe(&[terminal.clone()], true, &mut attention, now);
    let next = update(&terminal, FIRST);
    notifications.observe(&[next.clone()], true, &mut attention, now);
    assert!(notifications.take_bell().is_some());
    assert!(notifications.take_bell().is_none());
    assert!(notifications.message(now).unwrap().contains(BODY));
    assert!(notifications.message(now + EXPIRED).is_none());
    assert!(attention.contains(&terminal.fields().address));
    notifications.observe(&[next], true, &mut attention, now);
    assert!(notifications.take_bell().is_none());
}

#[test]
fn initial_attach_reconnect_regression_and_replaced_runtime_never_replay_alerts() {
    let terminal = fixtures::session();
    let mut notifications = Notifications::default();
    let mut attention = BTreeSet::new();
    let now = Instant::now();
    let first = update(&terminal, FIRST);
    let next = update(&terminal, NEXT);
    notifications.observe(&[first.clone()], true, &mut attention, now);
    assert!(notifications.take_bell().is_none());
    notifications.observe(&[next.clone()], false, &mut attention, now);
    notifications.observe(&[first], true, &mut attention, now);
    notifications.observe(&[next.clone()], true, &mut attention, now);
    assert!(notifications.take_bell().is_none());
    let mut fields = next.fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    notifications.observe(
        &[TerminalSummary::new(fields).unwrap()],
        true,
        &mut attention,
        now,
    );
    assert!(notifications.take_bell().is_none());
    assert!(attention.is_empty());
}

#[test]
fn remote_disconnection_and_catalog_removal_cannot_announce_retained_notifications() {
    let terminal = fixtures::session();
    let mut notifications = Notifications::default();
    let mut attention = BTreeSet::new();
    let now = Instant::now();
    notifications.observe(&[terminal.clone()], true, &mut attention, now);
    let mut stale = update(&terminal, FIRST);
    stale.mark_unreachable();
    notifications.observe(&[stale], true, &mut attention, now);
    assert!(notifications.take_bell().is_none());
    assert!(attention.is_empty());
    notifications.observe(&[update(&terminal, NEXT)], true, &mut attention, now);
    assert!(notifications.message(now).is_some());
    notifications.observe(&[], true, &mut attention, now);
    assert!(notifications.take_bell().is_none());
    assert!(notifications.message(now).is_none());
    assert!(attention.is_empty());
}
