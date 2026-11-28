use crate::host::tests::*;
use super::*;

const CREATION_IDENTITY_INCREMENT: i64 = 1;
const CREATION_IDENTITY_UNSIGNED_INCREMENT: u64 = 1;
const CREATION_APPEARING_LEDGER_NAME: &str = "creation-ledger-appearing";
const CREATION_EXISTING_LEDGER_NAME: &str = "creation-ledger-existing";
const CREATION_MISSING_EVIDENCE_PARENT: &str = "missing-evidence-parent";
const CREATION_MISSING_LEDGER_NAME: &str = "creation-ledger-missing";
const CREATION_REPLACED_LEDGER_NAME: &str = "creation-ledger-replaced";

#[test]
fn convergence_creation_evidence_treats_a_missing_parent_as_absent() {
    let root = tempdir().unwrap();
    let path = root
        .path()
        .join(CREATION_MISSING_EVIDENCE_PARENT)
        .join(CREATION_LEDGER_FILE_NAME);

    assert!(
        read_creation_evidence::<CreationLedger>(&path, CREATION_LEDGER_LABEL)
            .unwrap()
            .is_none()
    );
}

#[test]
fn convergence_one_durable_reader_observes_every_presence_transition() {
    let root = tempdir().unwrap();
    Paths::new(root.path()).ensure_base().unwrap();
    let existing = root.path().join(CREATION_EXISTING_LEDGER_NAME);
    let missing = root.path().join(CREATION_MISSING_LEDGER_NAME);
    let appearing = root.path().join(CREATION_APPEARING_LEDGER_NAME);
    write_creation_ledger(&existing, &CreationLedger::default()).unwrap();
    let publish = std::cell::Cell::new(false);
    let mut hook = |stage| {
        if publish.get() && stage == CreationDurableReadStage::MissingObserved {
            write_creation_ledger(&appearing, &CreationLedger::default())?;
        }
        Ok(())
    };

    assert!(read_convergence_ledger(&existing, &mut hook).is_some());
    assert!(read_convergence_ledger(&missing, &mut hook).is_none());
    publish.set(true);
    assert!(read_convergence_ledger(&appearing, &mut hook).is_some());
}

fn read_convergence_ledger<F>(path: &Path, hook: F) -> Option<CreationLedger>
where
    F: FnMut(CreationDurableReadStage) -> Result<()>,
{
    read_private_durable_json_with(path, CREATION_LEDGER_LABEL, MAX_CREATION_LEDGER_BYTES, hook)
        .unwrap()
}

#[test]
fn convergence_durable_creation_identity_verifies_each_captured_metadata_field() {
    let root = tempdir().unwrap();
    Paths::new(root.path()).ensure_base().unwrap();
    let path = root.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let metadata = std::fs::metadata(&path).unwrap();
    let identity = PrivateFileIdentity::inspect(
        &metadata,
        &path,
        CREATION_LEDGER_LABEL,
        MAX_CREATION_LEDGER_BYTES,
    )
    .unwrap();
    assert!(
        identity
            .verify(&metadata, &path, CREATION_LEDGER_LABEL)
            .is_ok()
    );

    let directory = std::fs::metadata(root.path()).unwrap();
    assert!(
        identity
            .verify(&directory, &path, CREATION_LEDGER_LABEL)
            .is_err()
    );

    for mutation in [
        CreationIdentityMutation::Device,
        CreationIdentityMutation::Inode,
        CreationIdentityMutation::Length,
        CreationIdentityMutation::ModifiedSeconds,
        CreationIdentityMutation::ModifiedNanoseconds,
        CreationIdentityMutation::ChangedSeconds,
        CreationIdentityMutation::ChangedNanoseconds,
    ] {
        let mut changed = identity;
        mutation.apply(&mut changed);
        assert!(
            changed
                .verify(&metadata, &path, CREATION_LEDGER_LABEL)
                .is_err()
        );
    }
}

#[test]
fn convergence_durable_creation_read_rejects_a_same_content_inode_replacement() {
    let root = tempdir().unwrap();
    Paths::new(root.path()).ensure_base().unwrap();
    let path = root.path().join(CREATION_LEDGER_FILE_NAME);
    let saved = root.path().join(CREATION_REPLACED_LEDGER_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let original = std::fs::read(&path).unwrap();

    let result = read_creation_ledger_with_test_hook(&path, |stage| {
        if stage == CreationDurableReadStage::AfterRead {
            std::fs::rename(&path, &saved)?;
            write_private_bytes(&path, &original);
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert!(saved.exists());
}

#[derive(Clone, Copy)]
enum CreationIdentityMutation {
    Device,
    Inode,
    Length,
    ModifiedSeconds,
    ModifiedNanoseconds,
    ChangedSeconds,
    ChangedNanoseconds,
}

impl CreationIdentityMutation {
    fn apply(self, identity: &mut PrivateFileIdentity) {
        match self {
            Self::Device => {
                identity.device += CREATION_IDENTITY_UNSIGNED_INCREMENT;
            }
            Self::Inode => {
                identity.inode += CREATION_IDENTITY_UNSIGNED_INCREMENT;
            }
            Self::Length => {
                identity.length += CREATION_IDENTITY_UNSIGNED_INCREMENT;
            }
            Self::ModifiedSeconds => {
                identity.modified_seconds += CREATION_IDENTITY_INCREMENT;
            }
            Self::ModifiedNanoseconds => {
                identity.modified_nanoseconds += CREATION_IDENTITY_INCREMENT;
            }
            Self::ChangedSeconds => {
                identity.changed_seconds += CREATION_IDENTITY_INCREMENT;
            }
            Self::ChangedNanoseconds => {
                identity.changed_nanoseconds += CREATION_IDENTITY_INCREMENT;
            }
        }
    }
}
