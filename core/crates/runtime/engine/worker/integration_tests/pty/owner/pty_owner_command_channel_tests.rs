use super::*;
use crate::worker::pty_owner_handle::PtyOwnerCommand;
use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;

const COMMAND_RACE_PARTICIPANTS: usize = 2;
const COMMAND_RACE_SIGNAL_CAPACITY: usize = 1;
const COMMAND_RACE_PENDING_PROBE: Duration = Duration::from_millis(20);

#[test]
fn successful_send_is_drained_when_close_races_admission() {
    let (sender, receiver) = pty_owner_command_channel();
    let (response, mut completion) = tokio::sync::oneshot::channel();
    let admitted = Arc::new(Barrier::new(COMMAND_RACE_PARTICIPANTS));
    let release = Arc::new(Barrier::new(COMMAND_RACE_PARTICIPANTS));
    let sender_thread = std::thread::spawn({
        let admitted = Arc::clone(&admitted);
        let release = Arc::clone(&release);
        move || {
            sender.send_after_admission_for_test(
                PtyOwnerCommand::Terminate {
                    force: true,
                    response,
                },
                || {
                    admitted.wait();
                    release.wait();
                },
            )
        }
    });
    admitted.wait();
    let (close_started, close_observed) = mpsc::sync_channel(COMMAND_RACE_SIGNAL_CAPACITY);
    let close_thread = std::thread::spawn(move || {
        close_started.send(()).unwrap();
        receiver.close_and_drain()
    });
    close_observed.recv().unwrap();
    release.wait();

    sender_thread.join().unwrap().unwrap();
    let mut drained = close_thread.join().unwrap();
    assert_eq!(drained.len(), 1);
    let PtyOwnerCommand::Terminate { force, response } = drained.pop().unwrap();
    assert!(force);
    response.send(Ok(())).unwrap();
    assert_eq!(completion.try_recv().unwrap(), Ok(()));
}

#[test]
fn send_after_close_is_rejected_with_its_response_custody() {
    let (sender, receiver) = pty_owner_command_channel();
    assert!(receiver.close_and_drain().is_empty());
    let (response, mut completion) = tokio::sync::oneshot::channel();

    let error = sender
        .send(PtyOwnerCommand::Terminate {
            force: false,
            response,
        })
        .unwrap_err();

    let PtyOwnerCommand::Terminate { response, .. } = error.0;
    response.send(Ok(())).unwrap();
    assert_eq!(completion.try_recv().unwrap(), Ok(()));
}

#[test]
fn send_blocked_after_drain_cannot_report_false_acceptance() {
    let (sender, receiver) = pty_owner_command_channel();
    let drained = Arc::new(Barrier::new(COMMAND_RACE_PARTICIPANTS));
    let release = Arc::new(Barrier::new(COMMAND_RACE_PARTICIPANTS));
    let close_thread = std::thread::spawn({
        let drained = Arc::clone(&drained);
        let release = Arc::clone(&release);
        move || {
            receiver.close_and_drain_after_for_test(|| {
                drained.wait();
                release.wait();
            })
        }
    });
    drained.wait();
    let (send_started, send_observed) = mpsc::sync_channel(COMMAND_RACE_SIGNAL_CAPACITY);
    let (send_finished, send_result) = mpsc::sync_channel(COMMAND_RACE_SIGNAL_CAPACITY);
    let send_thread = std::thread::spawn(move || {
        send_started.send(()).unwrap();
        let (response, completion) = tokio::sync::oneshot::channel();
        assert!(
            send_finished
                .send((
                    sender.send(PtyOwnerCommand::Terminate {
                        force: true,
                        response,
                    }),
                    completion,
                ))
                .is_ok()
        );
    });
    send_observed.recv().unwrap();
    assert!(matches!(
        send_result.recv_timeout(COMMAND_RACE_PENDING_PROBE),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    release.wait();

    assert!(close_thread.join().unwrap().is_empty());
    let (send, mut completion) = send_result.recv().unwrap();
    send_thread.join().unwrap();
    let PtyOwnerCommand::Terminate { response, .. } = send.unwrap_err().0;
    response.send(Ok(())).unwrap();
    assert_eq!(completion.try_recv().unwrap(), Ok(()));
}
