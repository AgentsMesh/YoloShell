use crate::host::tests::*;
use super::*;

fn create_creation_fifo(path: &Path) {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    let mode = libc::mode_t::try_from(CREATION_LEDGER_FILE_MODE).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), mode) }, 0);
}

fn assert_host_open_rejects_fifo(home: PathBuf, fifo: PathBuf) {
    create_creation_fifo(&fifo);
    let (sender, receiver) = std::sync::mpsc::channel();
    let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = ready_sender.send(());
        let rejected = Host::open(HostConfig::new(home)).is_err();
        let _ = sender.send(rejected);
    });
    ready_receiver.recv().unwrap();
    let rejected = receiver
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .expect("Host::open blocked while rejecting a FIFO");
    assert!(rejected);
    worker.join().unwrap();
}

#[test]
fn creation_ledger_fifo_is_rejected_without_blocking() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let ledger = directory.path().join(CREATION_LEDGER_FILE_NAME);
    assert_host_open_rejects_fifo(directory.path().into(), ledger);
}

#[test]
fn strictly_named_stale_creation_fifo_is_rejected_without_blocking() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let stale = directory.path().join(format!(
        ".{CREATION_LEDGER_FILE_NAME}.{}.tmp",
        Uuid::new_v4()
    ));
    assert_host_open_rejects_fifo(directory.path().into(), stale);
}

#[test]
fn strictly_named_stale_receipt_fifo_is_rejected_without_blocking() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let store = CreationReceiptStore::open(directory.path()).unwrap();
    let staging = store.staging.path.clone();
    drop(store);
    let stale = staging.join(format!(
        "{CREATION_RECEIPT_TEMP_PREFIX}{}{CREATION_RECEIPT_TEMP_SUFFIX}",
        Uuid::new_v4()
    ));
    assert_host_open_rejects_fifo(directory.path().into(), stale);
}
