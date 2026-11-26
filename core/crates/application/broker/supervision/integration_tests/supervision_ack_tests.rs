use super::*;
use std::time::Duration;

const TEST_TIMEOUT: Duration = Duration::from_secs(1);
const STALLED_COMMIT_TIMEOUT: Duration = Duration::from_millis(50);

#[test]
fn adoption_requires_prepare_and_commit_for_the_exact_instance() {
    let instance = Uuid::new_v4();
    let nonce = Uuid::new_v4();
    let deadline = Instant::now() + TEST_TIMEOUT;
    let (mut supervisor, broker) = BlockingLocalUnixStream::pair().unwrap();
    let (settled_send, settled_receive) = std::sync::mpsc::channel();
    let acknowledge = writer(&broker, nonce, deadline, settled_send).unwrap();
    let task = std::thread::spawn(move || acknowledge(instance));
    let prepared = read_prepared(&mut supervisor, nonce, instance, deadline).unwrap();
    commit(&mut supervisor, prepared, deadline).unwrap();
    assert!(task.join().unwrap());
    assert!(settled_receive.recv().unwrap());
}

#[test]
fn prepared_frame_rejects_a_different_instance() {
    let instance = Uuid::new_v4();
    let nonce = Uuid::new_v4();
    let (mut supervisor, broker) = BlockingLocalUnixStream::pair().unwrap();
    let deadline = Instant::now() + TEST_TIMEOUT;
    let (settled_send, settled_receive) = std::sync::mpsc::channel();
    let acknowledge = writer(&broker, nonce, deadline, settled_send).unwrap();
    let task = std::thread::spawn(move || acknowledge(instance));
    assert!(read_prepared(&mut supervisor, nonce, Uuid::new_v4(), deadline).is_err());
    drop(supervisor);
    assert!(!task.join().unwrap());
    assert!(!settled_receive.recv().unwrap());
}

#[test]
fn prepared_exchange_fails_closed_when_commit_never_arrives() {
    let instance = Uuid::new_v4();
    let nonce = Uuid::new_v4();
    let deadline = Instant::now() + STALLED_COMMIT_TIMEOUT;
    let (mut supervisor, broker) = BlockingLocalUnixStream::pair().unwrap();
    let (settled_send, settled_receive) = std::sync::mpsc::channel();
    let acknowledge = writer(&broker, nonce, deadline, settled_send).unwrap();
    let task = std::thread::spawn(move || acknowledge(instance));
    read_prepared(&mut supervisor, nonce, instance, deadline).unwrap();
    assert!(!task.join().unwrap());
    assert!(!settled_receive.recv().unwrap());
}
