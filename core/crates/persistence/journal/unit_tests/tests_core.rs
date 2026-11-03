use super::*;
use std::fs::OpenOptions;
use tempfile::tempdir;

#[test]
fn journal_replays_and_recovers_torn_tail_without_reusing_sequence() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    {
        let mut journal = Journal::open(&path, epoch).unwrap();
        assert_eq!(journal.append(b"one").unwrap().sequence, 1);
        assert_eq!(journal.append(b"two").unwrap().sequence, 2);
    }
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"OUT1partial")
        .unwrap();
    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.last_sequence(), 2);
    assert_eq!(
        reopened
            .replay_after(0, usize::MAX)
            .unwrap()
            .iter()
            .map(|chunk| chunk.data.clone())
            .collect::<Vec<_>>(),
        vec![b"one".to_vec(), b"two".to_vec()]
    );
    assert_eq!(reopened.append(b"three").unwrap().sequence, 3);
}

#[test]
fn epochs_have_independent_sequences_in_one_history_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    Journal::open(&path, first)
        .unwrap()
        .append(b"first")
        .unwrap();
    let mut second_journal = Journal::open(&path, second).unwrap();
    assert_eq!(second_journal.append(b"second").unwrap().sequence, 1);
    assert_eq!(Journal::open(&path, first).unwrap().last_sequence(), 1);
}

#[test]
fn compaction_persists_cursor_and_preserves_sequence() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(b"one").unwrap();
    journal.append(b"two").unwrap();
    journal.append(b"three").unwrap();
    journal.compact_through(2).unwrap();
    assert_eq!(journal.retained_from_sequence(), 3);
    assert_eq!(journal.last_sequence(), 3);
    assert_eq!(journal.replay_after(0, usize::MAX).unwrap().len(), 1);
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.retained_from_sequence(), 3);
    assert_eq!(reopened.append(b"four").unwrap().sequence, 4);
    assert!(
        !directory.path().read_dir().unwrap().flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(COMPACT_TEMPORARY_PREFIX)
        })
    );
}

#[test]
fn snapshot_round_trips_and_is_atomic() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("screen.snapshot");
    let snapshot = TerminalSnapshot {
        runtime_epoch: Uuid::new_v4(),
        barrier_sequence: 42,
        cols: 120,
        rows: 40,
        data: b"screen".to_vec(),
    };
    write_snapshot(&path, &snapshot).unwrap();
    let loaded = read_snapshot(&path).unwrap();
    assert_eq!(loaded.runtime_epoch, snapshot.runtime_epoch);
    assert_eq!(loaded.barrier_sequence, 42);
    assert_eq!(loaded.data, b"screen");
    assert!(!path.with_extension("snapshot.tmp").exists());
}
