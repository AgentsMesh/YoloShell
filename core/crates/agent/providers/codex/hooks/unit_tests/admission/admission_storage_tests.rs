use super::*;
use crate::agent::hooks::ProcessIdentity;
use std::sync::{Arc, Barrier};

#[path = "../../test_support/admission.rs"]
mod support;
use support::*;

const CONCURRENT_WRITERS: usize = 8;
const FOREIGN_DIRECTORY: &str = "foreign";
const ALTERNATE_EXECUTABLE: &str = "alternate-executable";
const PROCESS_PID_OFFSET: u32 = 1;
const PROCESS_START_OFFSET: u64 = 1;

#[test]
fn first_publication_is_durable_and_exact_retries_are_replays() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    let record = fixture.record();
    let created = storage.publish(&record).unwrap();
    assert!(!created.is_replay());
    assert_eq!(created.artifact().record(), &record);
    assert_eq!(storage.read().unwrap(), Some(created.artifact().clone()));
    storage.verify(created.artifact()).unwrap();

    let replay = storage.publish(&record).unwrap();
    assert!(replay.is_replay());
    assert_eq!(replay.artifact(), created.artifact());
    assert_eq!(
        replay.artifact().file_identity(),
        created.artifact().file_identity()
    );
    assert_eq!(replay.into_artifact(), created.into_artifact());
}

#[test]
fn all_concurrent_exact_publishers_converge_on_one_private_identity() {
    let fixture = Fixture::new();
    let storage = Arc::new(fixture.storage());
    let record = Arc::new(fixture.record());
    let barrier = Arc::new(Barrier::new(CONCURRENT_WRITERS));
    let handles: Vec<_> = (0..CONCURRENT_WRITERS)
        .map(|_| {
            let storage = Arc::clone(&storage);
            let record = Arc::clone(&record);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                storage.publish(&record).unwrap()
            })
        })
        .collect();
    let publications: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        publications
            .iter()
            .filter(|value| !value.is_replay())
            .count(),
        1
    );
    let expected = publications.first().unwrap().artifact();
    assert!(
        publications
            .iter()
            .all(|value| value.artifact() == expected)
    );
}

#[test]
fn every_valid_native_context_change_conflicts_for_the_same_invocation() {
    for &change in FieldChange::ALL {
        let fixture = Fixture::new();
        let storage = fixture.storage();
        storage.publish(&fixture.record()).unwrap();
        let changed = changed_record(&fixture, change);
        assert!(storage.publish(&changed).is_err(), "{change:?}");
    }
}

#[test]
fn storage_scope_must_match_the_complete_hook_identity() {
    let fixture = Fixture::new();
    let mut foreign = fixture.identity.clone();
    foreign.hook_token = uuid::Uuid::new_v4();
    assert!(CodexAdmissionStorage::for_identity(&fixture.artifacts, &foreign).is_ok());
    foreign.invocation_id = uuid::Uuid::new_v4();
    assert!(CodexAdmissionStorage::for_identity(&fixture.artifacts, &foreign).is_err());
}

#[derive(Clone, Copy, Debug)]
enum FieldChange {
    Session,
    Runtime,
    Token,
    ProcessPid,
    ProcessStart,
    Cwd,
    DataHome,
    HookPath,
    RealPath,
}

impl FieldChange {
    const ALL: &'static [Self] = &[
        Self::Session,
        Self::Runtime,
        Self::Token,
        Self::ProcessPid,
        Self::ProcessStart,
        Self::Cwd,
        Self::DataHome,
        Self::HookPath,
        Self::RealPath,
    ];
}

fn changed_record(fixture: &Fixture, change: FieldChange) -> CodexAdmissionRecord {
    let mut request = fixture.request.clone();
    match change {
        FieldChange::Session => request.identity.terminal_id = uuid::Uuid::new_v4(),
        FieldChange::Runtime => request.identity.runtime_epoch = uuid::Uuid::new_v4(),
        FieldChange::Token => request.identity.hook_token = uuid::Uuid::new_v4(),
        FieldChange::ProcessPid => {
            request.identity.process = ProcessIdentity::new(
                request.identity.process.pid + PROCESS_PID_OFFSET,
                request.identity.process.start_time,
            )
        }
        FieldChange::ProcessStart => {
            request.identity.process = ProcessIdentity::new(
                request.identity.process.pid,
                request.identity.process.start_time + PROCESS_START_OFFSET,
            )
        }
        FieldChange::Cwd => request.cwd = request.cwd.join(FOREIGN_DIRECTORY),
        FieldChange::DataHome => request.data_home = request.data_home.join(FOREIGN_DIRECTORY),
        FieldChange::HookPath => {
            request.hook_executable =
                fixture.alternate_executable(ALTERNATE_EXECUTABLE, EXECUTABLE_CONTENT)
        }
        FieldChange::RealPath => {
            request.real_executable =
                fixture.alternate_executable(ALTERNATE_EXECUTABLE, EXECUTABLE_CONTENT)
        }
    }
    CodexAdmissionRecord::capture(&request).unwrap()
}
