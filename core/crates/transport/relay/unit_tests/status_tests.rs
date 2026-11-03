use super::*;

#[test]
fn status_sink_preserves_every_transition_in_order() {
    let (sender, mut events) = mpsc::unbounded_channel();
    let sink = RelayStatusSink::from(sender);
    sink.send(RelayStatus::ControlAcquired);
    sink.send(RelayStatus::SessionEnding);
    sink.send(RelayStatus::SessionExited);
    assert_eq!(events.try_recv().unwrap(), RelayStatus::ControlAcquired);
    assert_eq!(events.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert_eq!(events.try_recv().unwrap(), RelayStatus::SessionExited);
}
