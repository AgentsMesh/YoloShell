use super::{Path, socket_path};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString};
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

#[path = "socket_stage_cleanup.rs"]
pub(crate) mod cleanup;
#[path = "socket_stage_cleanup_io.rs"]
mod cleanup_io;
#[path = "socket_stage_cleanup_lease.rs"]
mod cleanup_lease;
#[path = "socket_stage_lease.rs"]
mod lease;
#[path = "socket_stage_probe_cleanup.rs"]
mod probe_cleanup;
#[path = "socket_stage_probe_target.rs"]
mod probe_target;
#[path = "socket_stage_record.rs"]
mod record;
#[path = "socket_stage_residue_cleanup.rs"]
mod residue_cleanup;
#[cfg(test)]
pub(crate) use cleanup::cleanup_key;
pub(crate) use cleanup::{Cleanup, cleanup_key_for_parent, cleanup_lifecycle};
#[cfg(test)]
pub(crate) use cleanup::{cleanup_key_with_all_test_hooks, cleanup_key_with_test_hooks};
pub(crate) use lease::Lease;
#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use probe_cleanup::install_before_rename as install_probe_cleanup_rename_hook;
pub(crate) use residue_cleanup::cleanup_known as cleanup_residue;
const BIND_PREFIX: &str = ".b-";
const LEASE_PREFIX: &str = ".l-";
const PROBE_PREFIX: &str = ".p-";
const PROBE_QUARANTINE_PREFIX: &str = ".q-";
const LIFECYCLE_FILE_NAME: &str = ".stage.lock";
const DIGEST_SEPARATOR: u8 = 0;
const KEY_BYTES: usize = 16;
const KEY_HEX_LENGTH: usize = KEY_BYTES * crate::runtime_constants::HEX_DIGITS_PER_BYTE;
const FULL_DIGEST_HEX_LENGTH: usize = KEY_HEX_LENGTH + KEY_HEX_LENGTH;
const INVALID_STAGE_DIGEST: &str = "invalid stage digest";
const PLACEHOLDER_DIGEST_UNIT: &str = "0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Bind,
    Probe,
}

#[derive(Clone, Debug)]
pub(crate) struct Target {
    effective_user_id: u32,
    full_digest: String,
    key: String,
    parent: PathBuf,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Bind => BIND_PREFIX,
            Self::Probe => PROBE_PREFIX,
        }
    }
}

impl Target {
    pub(crate) fn new(parent: &Path, target: &CStr) -> Result<Self> {
        let effective_user_id = unsafe { libc::geteuid() };
        let full_digest = target_digest(parent, target, effective_user_id);
        Self::from_digest(parent, effective_user_id, full_digest)
    }

    fn from_digest(parent: &Path, effective_user_id: u32, full_digest: String) -> Result<Self> {
        let target = Self::from_existing_digest(parent, effective_user_id, full_digest)?;
        target.validate_paths()?;
        Ok(target)
    }

    fn from_existing_digest(
        parent: &Path,
        effective_user_id: u32,
        full_digest: String,
    ) -> Result<Self> {
        anyhow::ensure!(
            valid_hex(&full_digest, FULL_DIGEST_HEX_LENGTH),
            INVALID_STAGE_DIGEST
        );
        let key = full_digest[..KEY_HEX_LENGTH].to_owned();
        Ok(Self {
            effective_user_id,
            full_digest,
            key,
            parent: parent.to_path_buf(),
        })
    }

    pub(crate) fn stage(&self, kind: Kind) -> Result<(CString, PathBuf)> {
        named_path(&self.parent, kind.prefix(), &self.key)
    }

    pub(crate) fn lease(&self) -> Result<(CString, PathBuf)> {
        named_path(&self.parent, LEASE_PREFIX, &self.key)
    }

    pub(crate) fn lifecycle_path(&self) -> PathBuf {
        self.parent.join(LIFECYCLE_FILE_NAME)
    }

    pub(crate) fn probe_quarantine(&self) -> Result<(CString, PathBuf)> {
        named_path(&self.parent, PROBE_QUARANTINE_PREFIX, &self.key)
    }

    pub(crate) fn residue_names(&self) -> Result<Vec<CString>> {
        Ok(vec![
            self.stage(Kind::Bind)?.0,
            self.stage(Kind::Probe)?.0,
            self.probe_quarantine()?.0,
        ])
    }

    pub(crate) fn matches_endpoint(&self, name: &CStr) -> bool {
        target_digest(&self.parent, name, self.effective_user_id) == self.full_digest
    }

    #[cfg(test)]
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    fn validate_paths(&self) -> Result<()> {
        for kind in [Kind::Bind, Kind::Probe] {
            socket_path::validate(&self.stage(kind)?.1)?;
        }
        socket_path::validate(&self.lease()?.1)?;
        socket_path::validate(&self.probe_quarantine()?.1)?;
        Ok(())
    }
}

pub(crate) fn validate_parent(parent: &Path) -> Result<()> {
    let digest = PLACEHOLDER_DIGEST_UNIT.repeat(FULL_DIGEST_HEX_LENGTH);
    Target::from_digest(parent, unsafe { libc::geteuid() }, digest).map(|_| ())
}

pub(crate) fn is_managed_stage(name: &CStr) -> bool {
    [Kind::Bind, Kind::Probe]
        .into_iter()
        .any(|kind| key_after_prefix(name, kind.prefix()).is_some())
        || key_after_prefix(name, PROBE_QUARANTINE_PREFIX).is_some()
}

pub(crate) fn lease_key(name: &CStr) -> Option<&str> {
    key_after_prefix(name, LEASE_PREFIX)
}

pub(crate) fn is_lifecycle(name: &CStr) -> bool {
    name.to_bytes() == LIFECYCLE_FILE_NAME.as_bytes()
}

fn named_path(parent: &Path, prefix: &str, key: &str) -> Result<(CString, PathBuf)> {
    let name = CString::new(format!("{prefix}{key}"))?;
    let path = parent.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
    Ok((name, path))
}

fn key_after_prefix<'a>(name: &'a CStr, prefix: &str) -> Option<&'a str> {
    let value = name.to_str().ok()?.strip_prefix(prefix)?;
    valid_hex(value, KEY_HEX_LENGTH).then_some(value)
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn target_digest(parent: &Path, target: &CStr, effective_user_id: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(effective_user_id.to_be_bytes());
    hasher.update([DIGEST_SEPARATOR]);
    hasher.update(parent.as_os_str().as_bytes());
    hasher.update([DIGEST_SEPARATOR]);
    hasher.update(target.to_bytes());
    let mut encoded = String::with_capacity(FULL_DIGEST_HEX_LENGTH);
    for byte in hasher.finalize() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_stage/socket_stage_lease_tests.rs"]
mod lease_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_stage/socket_stage_tests.rs"]
mod tests;
