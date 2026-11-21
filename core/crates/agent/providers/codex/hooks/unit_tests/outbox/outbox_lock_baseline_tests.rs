use super::{envelope, identity};
use crate::codex_hooks::CodexOutbox;
use crate::codex_hooks::constants::{FIRST_SEQUENCE, OUTBOX_FILE_NAME};
use std::path::Path;
use uuid::Uuid;

const INVOCATION_DIRECTORY: &str = "instance-invocation";
const RETAINED_DIRECTORY: &str = "instance-retained-invocation";
const FOREIGN_FILE: &str = "foreign.json";
const DISPLACED_FILE: &str = "displaced.json";

#[test]
fn target_substitution_between_lock_and_load_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTBOX_FILE_NAME);
    let foreign = root.path().join(FOREIGN_FILE);
    let displaced = root.path().join(DISPLACED_FILE);
    let outbox = populated(&path);
    populated(&foreign);
    let original_bytes = std::fs::read(&path).unwrap();
    let foreign_bytes = std::fs::read(&foreign).unwrap();
    let path_for_hook = path.clone();
    let displaced_for_hook = displaced.clone();
    super::lock::set_after_acquire_hook(move || {
        std::fs::rename(&path_for_hook, displaced_for_hook).unwrap();
        std::fs::rename(foreign, path_for_hook).unwrap();
    });

    assert!(outbox.load().is_err());
    assert_eq!(std::fs::read(path).unwrap(), foreign_bytes);
    assert_eq!(std::fs::read(displaced).unwrap(), original_bytes);
}

#[test]
fn instance_never_rebinds_to_a_replacement_invocation_directory() {
    let root = tempfile::tempdir().unwrap();
    let invocation = root.path().join(INVOCATION_DIRECTORY);
    let retained = root.path().join(RETAINED_DIRECTORY);
    let path = invocation.join(OUTBOX_FILE_NAME);
    let owner = identity();
    let original = CodexOutbox::for_identity(&path, &owner.hook).unwrap();

    std::fs::rename(&invocation, &retained).unwrap();
    let replacement = CodexOutbox::for_identity(&path, &owner.hook).unwrap();
    let replacement_id = Uuid::new_v4();
    let committed = replacement
        .enqueue(envelope(&owner, replacement_id))
        .unwrap();

    assert_eq!(committed.sequence.0, FIRST_SEQUENCE);
    assert!(original.enqueue(envelope(&owner, Uuid::new_v4())).is_err());
    let state = replacement.load().unwrap();
    assert_eq!(
        state.pending.len(),
        usize::try_from(FIRST_SEQUENCE).unwrap()
    );
    assert_eq!(state.pending.front().unwrap().event_id, replacement_id);
    assert!(!retained.join(OUTBOX_FILE_NAME).exists());
}

fn populated(path: &Path) -> CodexOutbox {
    let outbox = CodexOutbox::new(path).unwrap();
    let owner = identity();
    outbox.enqueue(envelope(&owner, Uuid::new_v4())).unwrap();
    outbox
}
