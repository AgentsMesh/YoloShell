use super::test_support::{self, Fixture};
use std::fs::{self, FileTimes, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};

const REPLACEMENT_FILE: &str = "replacement.jsonl";
const MUTATED_FIRST_BYTE: u8 = b'[';
const APPENDED_RECORD: &str = "{}\n";

#[test]
fn hook_runs_between_the_single_fd_read_and_final_validation() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let called = AtomicBool::new(false);
    let result = fixture.inspect_with_hook(&evidence, || {
        called.store(true, Ordering::SeqCst);
    });
    assert!(called.load(Ordering::SeqCst));
    assert!(result.is_some());
}

#[test]
fn same_length_mutation_even_with_restored_mtime_is_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let original = fs::metadata(&evidence).unwrap();
    let result = fixture.inspect_with_hook(&evidence, || {
        let mut file = OpenOptions::new().write(true).open(&evidence).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&[MUTATED_FIRST_BYTE]).unwrap();
        file.set_times(FileTimes::new().set_modified(original.modified().unwrap()))
            .unwrap();
    });
    assert!(result.is_none());
}

#[test]
fn growth_and_deletion_after_read_are_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let appended = fixture.inspect_with_hook(&evidence, || {
        OpenOptions::new()
            .append(true)
            .open(&evidence)
            .unwrap()
            .write_all(APPENDED_RECORD.as_bytes())
            .unwrap();
    });
    assert!(appended.is_none());

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    assert!(
        fixture
            .inspect_with_hook(&evidence, || fs::remove_file(&evidence).unwrap())
            .is_none()
    );
}

#[test]
fn same_content_inode_replacement_after_read_is_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let replacement = fixture.rollout_directory.join(REPLACEMENT_FILE);
    let contents = fs::read(&evidence).unwrap();
    fs::write(&replacement, contents).unwrap();
    let result = fixture.inspect_with_hook(&evidence, || {
        fs::rename(&replacement, &evidence).unwrap();
    });
    assert!(result.is_none());
}
