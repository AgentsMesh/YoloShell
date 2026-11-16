use std::fs::OpenOptions;
use std::io::Write;
use tempfile::tempdir;
use uuid::Uuid;
use yolo_runtime::TerminalSnapshot;
use yolo_runtime::journal::{Journal, read_snapshot, write_snapshot};

#[test]
fn journal_recovers_a_torn_record_and_never_reuses_a_committed_sequence() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("journal.bin");
    let epoch = Uuid::new_v4();
    let committed_len;
    {
        let mut journal = Journal::open(&path, epoch).unwrap();
        assert_eq!(journal.append(b"alpha").unwrap().sequence, 1);
        assert_eq!(journal.append(b"beta").unwrap().sequence, 2);
        committed_len = std::fs::metadata(&path).unwrap().len();
    }
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"OUT1partial")
        .unwrap();
    let mut recovered = Journal::open(&path, epoch).unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().len(), committed_len);
    assert_eq!(recovered.append(b"gamma").unwrap().sequence, 3);
    let replay = recovered.replay_after(1, 1024).unwrap();
    assert_eq!(
        replay
            .iter()
            .map(|chunk| chunk.data.as_slice())
            .collect::<Vec<_>>(),
        vec![b"beta".as_slice(), b"gamma".as_slice()]
    );
}

#[test]
fn epochs_share_history_file_but_have_independent_cursors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("journal.bin");
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    Journal::open(&path, first)
        .unwrap()
        .append(b"first epoch")
        .unwrap();
    Journal::open(&path, second)
        .unwrap()
        .append(b"second epoch")
        .unwrap();
    assert_eq!(Journal::open(&path, first).unwrap().last_sequence(), 1);
    assert_eq!(Journal::open(&path, second).unwrap().last_sequence(), 1);
}

#[test]
fn corrupt_snapshot_is_rejected_without_affecting_journal() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("snapshot.bin");
    let snapshot = TerminalSnapshot {
        runtime_epoch: Uuid::new_v4(),
        barrier_sequence: 9,
        cols: 80,
        rows: 24,
        data: b"terminal".to_vec(),
    };
    write_snapshot(&path, &snapshot).unwrap();
    assert_eq!(read_snapshot(&path).unwrap().barrier_sequence, 9);
    let mut bytes = std::fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 0x80;
    std::fs::write(&path, bytes).unwrap();
    assert!(read_snapshot(&path).is_err());
}

#[test]
fn compacted_journal_stays_bounded_and_exposes_the_retained_cursor() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bounded.bin");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open_with_max_bytes(&path, epoch, 140).unwrap();
    journal.append(b"first-record-").unwrap();
    journal.append(b"second-record-").unwrap();
    assert!(journal.append(b"third-record-").is_err());

    journal.compact_through(2).unwrap();
    assert_eq!(journal.retained_from_sequence(), 3);
    assert_eq!(journal.append(b"third-record-").unwrap().sequence, 3);
    assert!(std::fs::metadata(&path).unwrap().len() <= journal.max_disk_bytes());

    let mut reopened = Journal::open_with_max_bytes(&path, epoch, 140).unwrap();
    assert_eq!(reopened.retained_from_sequence(), 3);
    assert_eq!(reopened.last_sequence(), 3);
    assert_eq!(
        reopened.replay_after(2, 1024).unwrap()[0].data,
        b"third-record-"
    );
}
