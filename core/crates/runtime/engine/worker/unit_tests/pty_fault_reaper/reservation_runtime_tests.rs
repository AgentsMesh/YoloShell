const PRIMARY_RECEIVER_TEST_CHANNEL_CAPACITY: usize = 1;
const REPLACEMENT_CLAIM_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn primary_receiver_drives_replacement_queued_before_vacant_claim_cancellation() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(PRIMARY_RECEIVER_TEST_CHANNEL_CAPACITY);
    reservation_runtime::spawn_receiver(receiver).unwrap();
    let control_plane = fault_reap_construction::vacant_fault_reap(
        FaultReapStateObserver::default(),
        schedule::FaultReapSchedule::production(),
    );
    let vacant = reservation_claim::FaultReapClaim::vacant(control_plane);
    sender.send(vacant.clone()).unwrap();
    let (replacement, completion, root) = driver_reaper();
    sender
        .send(reservation_claim::FaultReapClaim::new(replacement))
        .unwrap();

    vacant.cancel();
    drop(sender);

    let outcome = completion
        .receive_timeout(REPLACEMENT_CLAIM_TIMEOUT)
        .unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    assert_already_reaped(root);
}
