use super::*;
use std::sync::mpsc;
use std::time::Duration;

const FOREIGN_PROCESS_ID_INCREMENT: u32 = 1;
const FOREIGN_DROP_TIMEOUT: Duration = Duration::from_millis(200);
const FOREIGN_DROP_BLOCKED: &str = "foreign turn drop touched the process registry";

#[test]
fn foreign_turn_drop_never_waits_for_the_registry_mutex() {
    let directory = tempfile::tempdir().unwrap();
    let key = ObjectIdentity::from_metadata(&directory.path().metadata().unwrap());
    let process_id = std::process::id()
        .checked_add(FOREIGN_PROCESS_ID_INCREMENT)
        .unwrap();
    let turn = ProcessLockTurn::new(key, process_id);
    let registry = current_registry();
    let (completed, observed) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        drop(turn);
        completed.send(()).unwrap();
    });

    let result = observed.recv_timeout(FOREIGN_DROP_TIMEOUT);
    drop(registry);
    thread.join().unwrap();

    assert!(result.is_ok(), "{FOREIGN_DROP_BLOCKED}");
}
