use super::retirement_test_constants::{CHANNEL_CAPACITY, PRODUCER_WIN_OCCURRENCE_COUNT};
use super::retirement_test_support::RetirementFixture;
use crate::codex_hooks::CodexRetirementWriterFence;
use std::sync::mpsc::sync_channel;

#[test]
fn producer_holding_the_writer_fence_commits_before_sealing() {
    let fixture = RetirementFixture::new();
    let (acquired_tx, acquired_rx) = sync_channel(CHANNEL_CAPACITY);
    let (release_tx, release_rx) = sync_channel(CHANNEL_CAPACITY);

    let (producer, sealer) = std::thread::scope(|scope| {
        let producer = scope.spawn(|| {
            CodexRetirementWriterFence::set_after_acquire_hook(move || {
                acquired_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            fixture.intake().persist(fixture.envelope())
        });
        acquired_rx.recv().unwrap();
        let sealer = scope.spawn(|| {
            let writer = fixture.writer();
            writer.publish_seal(&fixture.seal(), fixture.admission())
        });
        release_tx.send(()).unwrap();
        (producer.join().unwrap(), sealer.join().unwrap())
    });

    producer.unwrap();
    sealer.unwrap();
    assert_eq!(
        fixture.intake().scan().unwrap().occurrences.len(),
        PRODUCER_WIN_OCCURRENCE_COUNT
    );
    assert!(fixture.seal_path().exists());
}

#[test]
fn sealer_holding_the_writer_fence_rejects_the_late_producer() {
    let fixture = RetirementFixture::new();
    let (acquired_tx, acquired_rx) = sync_channel(CHANNEL_CAPACITY);
    let (release_tx, release_rx) = sync_channel(CHANNEL_CAPACITY);

    let (sealer, producer) = std::thread::scope(|scope| {
        let sealer = scope.spawn(|| {
            CodexRetirementWriterFence::set_after_acquire_hook(move || {
                acquired_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            let writer = fixture.writer();
            writer.publish_seal(&fixture.seal(), fixture.admission())
        });
        acquired_rx.recv().unwrap();
        let producer = scope.spawn(|| fixture.intake().persist(fixture.envelope()));
        release_tx.send(()).unwrap();
        (sealer.join().unwrap(), producer.join().unwrap())
    });

    sealer.unwrap();
    assert!(producer.is_err());
    assert!(fixture.intake().scan().unwrap().occurrences.is_empty());
    assert!(fixture.seal_path().exists());
}
