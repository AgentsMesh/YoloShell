use crate::host::tests::*;
use super::*;

const INVALID_SESSION_DIRECTORY_NAME: &str = "not-a-session-uuid";
const CATALOG_EVIDENCE_SOCKET_NAME: &str = "catalog-evidence.sock";
pub(super) const CATALOG_GENERATION_FLOOR: u64 = 41;
const CATALOG_GENERATION_INCREMENT: u64 = 1;
const CATALOG_SCHEMA_OFFSET: u16 = 1;
const EXPECTED_REBUILT_MANIFESTS: usize = 1;
const FIRST_REBUILT_MANIFEST: usize = 0;
#[cfg(unix)]
const NON_UTF8_DIRECTORY_BYTE: u8 = 0xff;
#[cfg(unix)]
const FILESYSTEM_REJECTED_NAME_ERRORS: &[i32] = &[libc::EILSEQ, libc::EPERM];

#[derive(Clone, Copy, Debug)]
enum InvalidGenerationEvidence {
    RuntimeSchema,
    RuntimeSession,
    LaunchSchema,
    LaunchSession,
    TransactionSchema,
    TransactionSession,
}

fn reopened_generation_after_invalid_evidence(case: InvalidGenerationEvidence) -> (u64, u64) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let expected = host
        .controller_generation()
        .checked_add(CATALOG_GENERATION_INCREMENT)
        .unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    match case {
        InvalidGenerationEvidence::RuntimeSchema | InvalidGenerationEvidence::RuntimeSession => {
            let mut record = host_runtime_record(
                &host,
                manifest.terminal_id,
                directory.path().join(CATALOG_EVIDENCE_SOCKET_NAME),
            );
            record.controller_generation = CATALOG_GENERATION_FLOOR;
            match case {
                InvalidGenerationEvidence::RuntimeSchema => {
                    record.schema_version += CATALOG_SCHEMA_OFFSET
                }
                InvalidGenerationEvidence::RuntimeSession => record.terminal_id = Uuid::new_v4(),
                _ => unreachable!(),
            }
            atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();
        }
        InvalidGenerationEvidence::LaunchSchema | InvalidGenerationEvidence::LaunchSession => {
            let mut launch = shell_resume_launch(&host, &manifest);
            launch.controller_generation = CATALOG_GENERATION_FLOOR;
            match case {
                InvalidGenerationEvidence::LaunchSchema => {
                    launch.schema_version += CATALOG_SCHEMA_OFFSET
                }
                InvalidGenerationEvidence::LaunchSession => launch.terminal_id = Uuid::new_v4(),
                _ => unreachable!(),
            }
            atomic_write_json(&host.paths.worker_launch(manifest.terminal_id), &launch).unwrap();
        }
        InvalidGenerationEvidence::TransactionSchema
        | InvalidGenerationEvidence::TransactionSession => {
            let mut launch = shell_resume_launch(&host, &manifest);
            launch.controller_generation = CATALOG_GENERATION_FLOOR;
            let mut transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
            match case {
                InvalidGenerationEvidence::TransactionSchema => {
                    transaction.schema_version += CATALOG_SCHEMA_OFFSET
                }
                InvalidGenerationEvidence::TransactionSession => {
                    transaction.terminal_id = Uuid::new_v4()
                }
                _ => unreachable!(),
            }
            atomic_write_json(
                &host.paths.resume_transaction(manifest.terminal_id),
                &transaction,
            )
            .unwrap();
        }
    }
    drop(host);
    let reopened = Host::open(HostConfig::new(directory.path())).unwrap();
    (reopened.controller_generation(), expected)
}

#[test]
fn catalog_generation_ignores_every_mismatched_durable_identity() {
    for case in [
        InvalidGenerationEvidence::RuntimeSchema,
        InvalidGenerationEvidence::RuntimeSession,
        InvalidGenerationEvidence::LaunchSchema,
        InvalidGenerationEvidence::LaunchSession,
        InvalidGenerationEvidence::TransactionSchema,
        InvalidGenerationEvidence::TransactionSession,
    ] {
        let (actual, expected) = reopened_generation_after_invalid_evidence(case);
        assert_eq!(actual, expected, "generation accepted invalid {case:?}");
        assert!(actual < CATALOG_GENERATION_FLOOR);
    }
}

#[test]
fn valid_resume_transaction_contributes_its_generation_floor() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let mut launch = shell_resume_launch(&host, &manifest);
    launch.controller_generation = CATALOG_GENERATION_FLOOR;
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    atomic_write_json(
        &host.paths.resume_transaction(manifest.terminal_id),
        &transaction,
    )
    .unwrap();
    drop(host);

    let reopened = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(
        reopened.controller_generation(),
        CATALOG_GENERATION_FLOOR + CATALOG_GENERATION_INCREMENT
    );
}

#[test]
fn catalog_rebuild_ignores_non_session_and_manifestless_directories() {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    paths.ensure_base().unwrap();
    let valid = persist_rebuild_manifest(directory.path());
    std::fs::create_dir(paths.sessions_dir().join(INVALID_SESSION_DIRECTORY_NAME)).unwrap();
    paths.ensure_session(Uuid::new_v4()).unwrap();
    #[cfg(unix)]
    let non_utf8_result = std::fs::create_dir(
        paths
            .sessions_dir()
            .join(std::ffi::OsString::from_vec(vec![NON_UTF8_DIRECTORY_BYTE])),
    );
    #[cfg(unix)]
    if let Err(error) = non_utf8_result {
        assert!(
            error
                .raw_os_error()
                .is_some_and(|code| FILESYSTEM_REJECTED_NAME_ERRORS.contains(&code))
        );
    }

    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifests = lock_recover(&host.catalog).list(true).unwrap();
    assert_eq!(manifests.len(), EXPECTED_REBUILT_MANIFESTS);
    assert_eq!(
        manifests[FIRST_REBUILT_MANIFEST].terminal_id,
        valid.terminal_id
    );
}

#[derive(Clone, Copy, Debug)]
enum InvalidManifestIdentity {
    Schema,
    Session,
}

#[test]
fn catalog_rebuild_rejects_every_manifest_identity_mismatch() {
    for case in [
        InvalidManifestIdentity::Schema,
        InvalidManifestIdentity::Session,
    ] {
        let directory = tempdir().unwrap();
        let paths = Paths::new(directory.path());
        let expected_id = Uuid::new_v4();
        paths.ensure_session(expected_id).unwrap();
        let mut manifest = fixture_manifest(directory.path());
        manifest.terminal_id = expected_id;
        match case {
            InvalidManifestIdentity::Schema => manifest.schema_version += CATALOG_SCHEMA_OFFSET,
            InvalidManifestIdentity::Session => manifest.terminal_id = Uuid::new_v4(),
        }
        atomic_write_json(&paths.manifest(expected_id), &manifest).unwrap();

        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        assert!(lock_recover(&host.catalog).list(true).unwrap().is_empty());
    }
}
