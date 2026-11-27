use super::*;

const BASE_UNSIGNED_VALUE: u64 = 17;
const BASE_SIGNED_VALUE: i64 = 23;
const VALUE_INCREMENT: u64 = 1;
const TIME_INCREMENT: i64 = 1;
const STABLE_READ_LENGTH: usize = BASE_UNSIGNED_VALUE as usize;
const DIFFERENT_READ_LENGTH: usize = STABLE_READ_LENGTH + VALUE_INCREMENT as usize;

#[test]
fn snapshot_match_accepts_only_complete_identity_and_length_equality() {
    let snapshot = base_snapshot();
    assert!(snapshots_match(
        snapshot,
        snapshot,
        snapshot,
        STABLE_READ_LENGTH
    ));

    for mutate in after_mutations() {
        let mut after = snapshot;
        mutate(&mut after);
        assert!(!snapshots_match(
            snapshot,
            after,
            snapshot,
            STABLE_READ_LENGTH
        ));
    }

    let mut path = snapshot;
    increment_device(&mut path);
    assert!(!snapshots_match(
        snapshot,
        snapshot,
        path,
        STABLE_READ_LENGTH
    ));
    let mut path = snapshot;
    increment_inode(&mut path);
    assert!(!snapshots_match(
        snapshot,
        snapshot,
        path,
        STABLE_READ_LENGTH
    ));
    assert!(!snapshots_match(
        snapshot,
        snapshot,
        snapshot,
        DIFFERENT_READ_LENGTH
    ));
}

fn base_snapshot() -> FileSnapshot {
    FileSnapshot {
        device: BASE_UNSIGNED_VALUE,
        inode: BASE_UNSIGNED_VALUE,
        length: BASE_UNSIGNED_VALUE,
        modified_seconds: BASE_SIGNED_VALUE,
        modified_nanoseconds: BASE_SIGNED_VALUE,
        changed_seconds: BASE_SIGNED_VALUE,
        changed_nanoseconds: BASE_SIGNED_VALUE,
    }
}

fn after_mutations() -> [fn(&mut FileSnapshot); 7] {
    [
        increment_device,
        increment_inode,
        increment_length,
        increment_modified_seconds,
        increment_modified_nanoseconds,
        increment_changed_seconds,
        increment_changed_nanoseconds,
    ]
}

fn increment_device(snapshot: &mut FileSnapshot) {
    snapshot.device += VALUE_INCREMENT;
}

fn increment_inode(snapshot: &mut FileSnapshot) {
    snapshot.inode += VALUE_INCREMENT;
}

fn increment_length(snapshot: &mut FileSnapshot) {
    snapshot.length += VALUE_INCREMENT;
}

fn increment_modified_seconds(snapshot: &mut FileSnapshot) {
    snapshot.modified_seconds += TIME_INCREMENT;
}

fn increment_modified_nanoseconds(snapshot: &mut FileSnapshot) {
    snapshot.modified_nanoseconds += TIME_INCREMENT;
}

fn increment_changed_seconds(snapshot: &mut FileSnapshot) {
    snapshot.changed_seconds += TIME_INCREMENT;
}

fn increment_changed_nanoseconds(snapshot: &mut FileSnapshot) {
    snapshot.changed_nanoseconds += TIME_INCREMENT;
}
