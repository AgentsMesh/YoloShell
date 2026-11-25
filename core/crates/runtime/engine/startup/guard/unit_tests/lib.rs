use super::*;
use constants::*;
use tokio::{
    io::AsyncWriteExt,
    time::{Duration, timeout},
};

const BUDGET: Duration = Duration::from_millis(100);
const CHECK_BUDGET: Duration = Duration::from_secs(2);
const WRONG_PROCESS: u32 = u32::MAX;
const MALFORMED: &[u8] = b"not a startup frame\n";

fn pair() -> (Sender, Receiver) {
    let (sender, stream) = channel().unwrap();
    let stream = std::os::unix::net::UnixStream::from(stream);
    stream.set_nonblocking(true).unwrap();
    (
        sender,
        Receiver {
            stream: UnixStream::from_std(stream).unwrap(),
            deadline: Instant::now() + BUDGET,
        },
    )
}

#[tokio::test]
async fn committed_service_survives_parent_eof_and_original_startup_deadline() {
    let (sender, receiver) = pair();
    let mut service = tokio::spawn(receiver.cancellation());
    sender
        .commit(std::process::id(), Instant::now() + CHECK_BUDGET)
        .await
        .unwrap();
    assert!(timeout(BUDGET + BUDGET, &mut service).await.is_err());
    service.abort();
    assert!(service.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn eof_wrong_target_and_timeout_cancel_uncommitted_services() {
    let (sender, receiver) = pair();
    drop(sender);
    timeout(CHECK_BUDGET, receiver.cancellation())
        .await
        .unwrap();
    let (sender, receiver) = pair();
    let service = tokio::spawn(receiver.cancellation());
    assert!(
        sender
            .commit(WRONG_PROCESS, Instant::now() + CHECK_BUDGET)
            .await
            .is_err()
    );
    timeout(CHECK_BUDGET, service).await.unwrap().unwrap();
    let (_sender, receiver) = pair();
    timeout(CHECK_BUDGET, receiver.cancellation())
        .await
        .unwrap();
}

#[tokio::test]
async fn malformed_partial_and_oversized_frames_never_commit() {
    for payload in [
        MALFORMED.to_vec(),
        vec![INVALID_FRAME_BYTE; MAX_FRAME + SENTINEL],
        vec![b'x'; MAX_FRAME + SENTINEL],
    ] {
        let (mut sender, receiver) = pair();
        sender.stream.write_all(&payload).await.unwrap();
        drop(sender);
        timeout(CHECK_BUDGET, receiver.cancellation())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn a_lost_acknowledgement_does_not_revoke_an_accepted_commit() {
    let (mut sender, receiver) = pair();
    frame::write(
        &mut sender.stream,
        Frame::new(std::process::id(), Phase::Commit),
    )
    .await
    .unwrap();
    drop(sender);
    assert!(
        timeout(BUDGET + BUDGET, receiver.cancellation())
            .await
            .is_err()
    );
}

#[test]
fn frame_validation_binds_version_target_and_direction() {
    let pid = std::process::id();
    assert!(
        Frame::new(pid, Phase::Commit)
            .validate(pid, Phase::Commit)
            .is_ok()
    );
    assert!(
        Frame::new(pid, Phase::Committed)
            .validate(pid, Phase::Commit)
            .is_err()
    );
    assert!(
        Frame::new(0, Phase::Commit)
            .validate(0, Phase::Commit)
            .is_err()
    );
    assert!(
        Frame {
            version: VERSION + 1,
            process_id: pid,
            phase: Phase::Commit
        }
        .validate(pid, Phase::Commit)
        .is_err()
    );
}

#[tokio::test]
async fn expired_sender_and_receiver_cannot_commit_already_buffered_frames() {
    let (sender, receiver) = pair();
    assert!(
        sender
            .commit(std::process::id(), Instant::now())
            .await
            .is_err()
    );
    timeout(CHECK_BUDGET, receiver.cancellation())
        .await
        .unwrap();
    let (mut sender, mut receiver) = pair();
    frame::write(
        &mut sender.stream,
        Frame::new(std::process::id(), Phase::Commit),
    )
    .await
    .unwrap();
    receiver.deadline = Instant::now();
    drop(sender);
    timeout(CHECK_BUDGET, receiver.cancellation())
        .await
        .unwrap();
}
