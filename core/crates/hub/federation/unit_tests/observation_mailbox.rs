use super::*;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;

#[test]
fn pending_snapshots_coalesce_without_changing_the_inflight_snapshot() {
    let mailbox = Mailbox::default();
    mailbox.available.store(true, Ordering::Release);
    let entry = Identity::new().identity;
    let first = ObservationHistory::new(entry, Vec::new(), routing_limits()).unwrap();
    let origin = yolo_hub_peer_protocol_test_support::Fixture::new();
    let newest =
        ObservationHistory::new(entry, vec![origin.export.catalogue], routing_limits()).unwrap();
    let first_receipt = mailbox.record(first.clone()).unwrap();
    let active = mailbox.next().unwrap();
    let mut replaced_receipt = mailbox.record(first.clone()).unwrap();
    let latest_receipt = mailbox.record(newest.clone()).unwrap();
    assert_eq!(active.history, first);
    assert!(matches!(
        replaced_receipt.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    ));
    let pending = mailbox.next().unwrap();
    assert_eq!(pending.history, newest);
    assert!(mailbox.pending.lock().unwrap().is_none());
    active.completed.send(()).unwrap();
    pending.completed.send(()).unwrap();
    drop((first_receipt, latest_receipt));
}

#[test]
fn retirement_discards_pending_work_and_rejects_all_later_snapshots() {
    let mailbox = Mailbox::default();
    mailbox.available.store(true, Ordering::Release);
    let history =
        ObservationHistory::new(Identity::new().identity, Vec::new(), routing_limits()).unwrap();
    let mut receipt = mailbox.record(history.clone()).unwrap();
    mailbox.retire();
    assert!(mailbox.retirement.is_retired());
    assert!(!mailbox.available.load(Ordering::Acquire));
    assert!(mailbox.next().is_none());
    assert!(mailbox.record(history).is_none());
    assert!(matches!(
        receipt.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    ));
}
