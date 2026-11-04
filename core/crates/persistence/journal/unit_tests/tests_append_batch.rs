use super::*;
use tempfile::tempdir;

#[test]
fn batch_append_publishes_every_record_and_replays_in_order() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    let payloads = [b"first".as_slice(), b"second".as_slice(), b"third".as_slice()];

    let chunks = journal.append_batch(&payloads).unwrap();

    assert_eq!(
        chunks.iter().map(|chunk| chunk.sequence).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(journal.last_sequence(), 3);
    assert_eq!(journal.replay_after(0, usize::MAX).unwrap().len(), payloads.len());
    drop(journal);
    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.replay_after(0, usize::MAX).unwrap().len(), payloads.len());
    assert_eq!(reopened.append(b"four").unwrap().sequence, 4);
}

#[test]
fn interrupted_batch_record_sync_reopens_without_a_sequence_gap() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal
        .append_batch(&[b"before".as_slice(), b"batch".as_slice()])
        .unwrap();
    journal.inject_append_failure(AppendStage::AfterRecordSync);
    let error = journal
        .append_batch(&[b"after".as_slice(), b"still-pending".as_slice()])
        .unwrap_err();
    assert!(error.to_string().contains("injected journal append uncertainty"));
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.last_sequence(), 2);
    assert_eq!(reopened.append(b"next").unwrap().sequence, 3);
}

#[test]
fn uncertain_batch_cursor_publication_reopens_with_the_batch_once() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.inject_append_failure(AppendStage::AfterCursorCommit);
    let error = journal
        .append_batch(&[b"first".as_slice(), b"second".as_slice()])
        .unwrap_err();
    assert!(error.to_string().contains("injected journal append uncertainty"));
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    let chunks = reopened.replay_after(0, usize::MAX).unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].data, b"first");
    assert_eq!(chunks[1].data, b"second");
    assert_eq!(reopened.append(b"third").unwrap().sequence, 3);
}
