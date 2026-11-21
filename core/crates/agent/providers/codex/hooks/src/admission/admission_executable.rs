use anyhow::{Context, Result, ensure};
use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "admission_executable_identity.rs"]
mod identity_record;
#[path = "admission_executable_kind.rs"]
mod kind;
pub use identity_record::CodexExecutableIdentity;
use identity_record::capture as identity;
pub use kind::CodexExecutableKind;

include!("admission_executable_mapping.rs");

const CONTENT_DIGEST_DOMAIN: &str = "yolo.codex-admission.executable.v1";
const DIGEST_CHUNK_BYTES: usize = 64 * 1_024;
const EXECUTABLE_OPEN_FLAGS: i32 =
    libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY;
const RAW_SHA256_BYTES: usize = ring::digest::SHA256_OUTPUT_LEN;
const EXECUTABLE_CHANGED: &str = "Codex admission executable changed while captured";
const EXECUTABLE_INVALID: &str = "Codex admission executable is invalid";
const PROVIDER_HASH_TIMEOUT: &str = "timed out hashing the Codex provider executable";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexExecutableAttestation {
    path: PathBuf,
    identity: CodexExecutableIdentity,
    #[serde(with = "crate::sha256_digest::compact")]
    content_digest: crate::sha256_digest::Sha256Digest,
}

impl CodexExecutableAttestation {
    pub(crate) fn capture(path: &Path, kind: CodexExecutableKind) -> Result<Self> {
        Self::capture_open(path, kind).map(|(attestation, _)| attestation)
    }

    fn capture_open(path: &Path, kind: CodexExecutableKind) -> Result<(Self, File)> {
        validate_bound_path(path)?;
        ensure!(std::fs::canonicalize(path)? == path, EXECUTABLE_INVALID);
        let named = identity(&std::fs::symlink_metadata(path)?, kind)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(EXECUTABLE_OPEN_FLAGS)
            .open(path)
            .context(EXECUTABLE_INVALID)?;
        let opened = identity(&file.metadata()?, kind)?;
        ensure!(named == opened, EXECUTABLE_CHANGED);
        let deadline = (kind == CodexExecutableKind::Provider)
            .then(|| Instant::now() + crate::codex_hooks::CODEX_MANAGEMENT_HANDSHAKE_TIMEOUT);
        let raw_digest = content_digest(&mut file, opened.length(), deadline)?;
        let content_digest =
            crate::sha256_digest::Sha256Digest::capture(&(CONTENT_DIGEST_DOMAIN, raw_digest))?;
        let after = identity(&file.metadata()?, kind)?;
        let renamed = identity(&std::fs::symlink_metadata(path)?, kind)?;
        ensure!(opened == after && after == renamed, EXECUTABLE_CHANGED);
        Ok((
            Self {
                path: path.to_path_buf(),
                identity: opened,
                content_digest,
            },
            file,
        ))
    }

    pub(crate) fn validate_for(&self, path: &Path, kind: CodexExecutableKind) -> Result<()> {
        ensure!(self.path == path, EXECUTABLE_INVALID);
        validate_bound_path(path)?;
        self.identity.validate(kind)
    }

    pub fn verify_current(&self, kind: CodexExecutableKind) -> Result<()> {
        ensure!(
            Self::capture(&self.path, kind)? == *self,
            EXECUTABLE_CHANGED
        );
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn identity(&self) -> CodexExecutableIdentity {
        self.identity
    }
    pub(crate) fn content_digest(&self) -> crate::sha256_digest::Sha256Digest {
        self.content_digest
    }
}

fn validate_bound_path(path: &Path) -> Result<()> {
    crate::paths::validation::validate_absolute(path)?;
    ensure!(
        path.as_os_str().as_bytes().len() <= crate::constants::MAX_RUNTIME_PATH_BYTES,
        EXECUTABLE_INVALID
    );
    Ok(())
}

fn content_digest(
    file: &mut File,
    length: u64,
    deadline: Option<Instant>,
) -> Result<[u8; RAW_SHA256_BYTES]> {
    let mut digest = DigestContext::new(&SHA256);
    let mut buffer = vec![u8::default(); DIGEST_CHUNK_BYTES];
    let mut input = file.take(length);
    loop {
        if deadline.is_some_and(|end| Instant::now() >= end) {
            return Err(std::io::Error::new(ErrorKind::TimedOut, PROVIDER_HASH_TIMEOUT).into());
        }
        if input.limit() == 0 {
            return Ok(digest.finish().as_ref().try_into()?);
        }
        let read = input.read(&mut buffer)?;
        ensure!(read != usize::default(), EXECUTABLE_CHANGED);
        digest.update(&buffer[..read]);
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_digest_tests.rs"]
mod digest_tests;
