#[test]
fn waited_lock_rejects_same_length_rewrite_with_restored_mtime() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE_NAME);
    let mut initial = PrivateFileLock::try_acquire(&path, MUTABLE_LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    initial.replace_contents(INITIAL_LOCK_CONTENT).unwrap();
    drop(initial);
    let entry = PrivateFileEntry::open(&path).unwrap();
    let mut opened = entry.open_rw(MUTABLE_LOCK_MAX_BYTES).unwrap();
    let original = opened.identity;
    let modified = opened.file.metadata().unwrap().modified().unwrap();
    assert_eq!(
        unsafe { libc::flock(opened.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        FLOCK_SUCCESS
    );
    let (observed_tx, observed_rx) = mpsc::channel();
    let waiter_path = path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        let directory = PrivateDirectory::open(waiter_path.parent().unwrap()).unwrap();
        PrivateFileLock::acquire_in_wait(
            directory,
            &waiter_path,
            MUTABLE_LOCK_MAX_BYTES,
            CONTENTION_TIMEOUT,
            RETRY_DELAY,
        )
    });
    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    opened.file.rewind().unwrap();
    opened.file.write_all(REPLACEMENT_LOCK_CONTENT).unwrap();
    opened.file.sync_all().unwrap();
    opened
        .file
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let changed =
        FileIdentity::inspect_file_bounded(&opened.file, &path, MUTABLE_LOCK_MAX_BYTES).unwrap();
    assert!(changed.same_content_version(original));
    assert!(!changed.unchanged(original));
    drop(opened);
    assert!(waiter.join().unwrap().is_err());
}
