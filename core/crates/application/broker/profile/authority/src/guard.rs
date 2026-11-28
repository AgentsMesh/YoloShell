use anyhow::{Result, ensure};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(test)]
use crate::digest;
use crate::document::{AuthorityDocument, validate_digest};
use crate::secure_file::{AuthorityLock, FileIdentity, read_document};

const AUTHORITY_DOCUMENT_SUFFIX: &str = ".authority.json";
const AUTHORITY_LOCK_SUFFIX: &str = ".authority.lock";
const MAXIMUM_AUTHORITY_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;

pub struct StartAuthorityGuard {
    guard: HostAuthorityGuard,
    host_id: String,
    profile_sha256: String,
}

pub struct RetirementAuthorityGuard {
    _guard: HostAuthorityGuard,
}

struct HostAuthorityGuard {
    lock: AuthorityLock,
    document_path: PathBuf,
    document_identity: FileIdentity,
    expectation: AuthorityExpectation,
}

enum AuthorityExpectation {
    Committed {
        host_id: String,
        profile_sha256: String,
    },
    Blocked {
        host_id: String,
        mutation_id: Uuid,
    },
}

impl StartAuthorityGuard {
    pub fn acquire(document: &Path, lock: &Path, expected_sha256: &str) -> Result<Self> {
        validate_digest(expected_sha256)?;
        let lock_handle = AuthorityLock::acquire(lock)?;
        let (authority, document_identity) = load_document(document)?;
        let host_id = authority.require_committed(expected_sha256)?.to_owned();
        validate_namespace(document, lock, &host_id)?;
        let guard = HostAuthorityGuard {
            lock: lock_handle,
            document_path: document.to_owned(),
            document_identity,
            expectation: AuthorityExpectation::Committed {
                host_id: host_id.clone(),
                profile_sha256: expected_sha256.to_owned(),
            },
        };
        guard.verify()?;
        Ok(Self {
            guard,
            host_id,
            profile_sha256: expected_sha256.to_owned(),
        })
    }

    pub fn verify(&self) -> Result<()> {
        self.guard.verify()
    }

    pub fn verify_config(&self, host_id: &str, profile_sha256: &str) -> Result<()> {
        self.verify()?;
        ensure!(
            host_id == self.host_id,
            "broker config host does not match authority"
        );
        ensure!(
            profile_sha256 == self.profile_sha256,
            "broker config authority digest mismatch"
        );
        Ok(())
    }
}

impl RetirementAuthorityGuard {
    pub fn acquire(document: &Path, lock: &Path, host_id: &str, mutation_id: Uuid) -> Result<Self> {
        let lock_handle = AuthorityLock::acquire(lock)?;
        validate_namespace(document, lock, host_id)?;
        let (document_value, document_identity) = load_document(document)?;
        document_value.require_blocked(host_id, mutation_id)?;
        let guard = HostAuthorityGuard {
            lock: lock_handle,
            document_path: document.to_owned(),
            document_identity,
            expectation: AuthorityExpectation::Blocked {
                host_id: host_id.to_owned(),
                mutation_id,
            },
        };
        guard.verify()?;
        Ok(Self { _guard: guard })
    }
}

impl HostAuthorityGuard {
    fn verify(&self) -> Result<()> {
        self.lock.verify()?;
        let (document, identity) = load_document(&self.document_path)?;
        ensure!(
            identity == self.document_identity,
            "remote host authority document identity changed"
        );
        match &self.expectation {
            AuthorityExpectation::Committed {
                host_id,
                profile_sha256,
            } => {
                ensure!(
                    document.require_committed(profile_sha256)? == host_id,
                    "remote host authority host changed"
                );
            }
            AuthorityExpectation::Blocked {
                host_id,
                mutation_id,
            } => {
                document.require_blocked(host_id, *mutation_id)?;
            }
        }
        Ok(())
    }
}

fn load_document(path: &Path) -> Result<(AuthorityDocument, FileIdentity)> {
    let (bytes, identity) = read_document(path, MAXIMUM_AUTHORITY_DOCUMENT_BYTES)?;
    Ok((AuthorityDocument::decode(&bytes)?, identity))
}

fn validate_namespace(document: &Path, lock: &Path, host_id: &str) -> Result<()> {
    ensure!(
        document.parent() == lock.parent(),
        "authority document and lock directories differ"
    );
    let document_name = format!("{host_id}{AUTHORITY_DOCUMENT_SUFFIX}");
    let lock_name = format!("{host_id}{AUTHORITY_LOCK_SUFFIX}");
    ensure!(
        document.file_name() == Some(OsStr::new(&document_name)),
        "authority document name mismatch"
    );
    ensure!(
        lock.file_name() == Some(OsStr::new(&lock_name)),
        "authority lock name mismatch"
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_tests.rs"]
mod integration_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/fixture.rs"]
mod test_fixture;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod unit_tests;
