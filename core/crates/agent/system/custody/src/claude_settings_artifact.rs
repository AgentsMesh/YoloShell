use std::ffi::OsStr;
use uuid::Uuid;

pub const DIRECTORY_NAME: &str = "claude-settings";
pub const MAX_ARTIFACTS: usize = 64;
pub const MAINTENANCE_LOCK_FILE_NAME: &str = ".yolo-artifact-maintenance";
use crate::claude_hooks::{CLAUDE_SETTINGS_FILE_PREFIX, CLAUDE_SETTINGS_FILE_SUFFIX};
#[path = "claude_settings_failure_marker.rs"]
mod failure_marker;
pub use failure_marker::{create_lost_marker, lost_marker_matches};
const FIELD_SEPARATOR: char = '_';
const KNOWN_BOOT: &str = "k";
const UNKNOWN_BOOT: &str = "u";
pub const HANDSHAKE_FAILURE_SUFFIX: &str = ".handshake-failed";
const BOOT_IDENTITY_UNAVAILABLE: &str = "Claude management requires a stable boot identity";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Owner {
    boot: crate::process::boot_identity::BootIdentity,
    pid: u32,
    start_time: u64,
    invocation_id: Uuid,
}

impl Owner {
    pub fn current(invocation_id: Uuid) -> anyhow::Result<Self> {
        let pid = std::process::id();
        let boot_id = crate::process::boot_id();
        require_known_boot(&boot_id)?;
        Ok(Self::from_values(
            &boot_id,
            pid,
            crate::process::process_start_time(pid)?,
            invocation_id,
        ))
    }

    pub fn for_process(pid: u32, start_time: u64, invocation_id: Uuid) -> Self {
        Self::from_values(&crate::process::boot_id(), pid, start_time, invocation_id)
    }

    pub fn for_boot_process(
        boot_id: &str,
        pid: u32,
        start_time: u64,
        invocation_id: Uuid,
    ) -> anyhow::Result<Self> {
        require_known_boot(boot_id)?;
        Ok(Self::from_values(boot_id, pid, start_time, invocation_id))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn for_test(pid: u32, start_time: u64, invocation_id: Uuid) -> Self {
        Self::for_process(pid, start_time, invocation_id)
    }

    fn from_values(boot_id: &str, pid: u32, start_time: u64, invocation_id: Uuid) -> Self {
        Self {
            boot: crate::process::boot_identity::BootIdentity::from_value(boot_id),
            pid,
            start_time,
            invocation_id,
        }
    }

    pub fn file_name(&self) -> String {
        let boot = if self.boot.known() {
            KNOWN_BOOT
        } else {
            UNKNOWN_BOOT
        };
        format!(
            "{CLAUDE_SETTINGS_FILE_PREFIX}{boot}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}{CLAUDE_SETTINGS_FILE_SUFFIX}",
            self.boot.encoded(),
            self.pid,
            self.start_time,
            self.invocation_id
        )
    }

    pub fn handshake_failure_file_name(&self) -> String {
        format!("{}{HANDSHAKE_FAILURE_SUFFIX}", self.file_name())
    }

    pub fn parse_artifact(value: &OsStr) -> Option<Self> {
        if let Some(owner) = Self::parse(value) {
            return (OsStr::new(&owner.file_name()) == value).then_some(owner);
        }
        let original = value.to_str()?.strip_suffix(HANDSHAKE_FAILURE_SUFFIX)?;
        let owner = Self::parse(OsStr::new(original))?;
        (OsStr::new(&owner.handshake_failure_file_name()) == value).then_some(owner)
    }

    pub fn is_handshake_failure(value: &OsStr) -> bool {
        value
            .to_str()
            .is_some_and(|name| name.ends_with(HANDSHAKE_FAILURE_SUFFIX))
    }

    pub fn invocation_id(&self) -> Uuid {
        self.invocation_id
    }

    pub fn process_identity(&self) -> crate::process::ProcessIdentity {
        crate::process::ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
    }

    pub fn parse(value: &OsStr) -> Option<Self> {
        let original = value.to_str()?;
        let value = original
            .strip_prefix(CLAUDE_SETTINGS_FILE_PREFIX)?
            .strip_suffix(CLAUDE_SETTINGS_FILE_SUFFIX)?;
        let fields = value.split(FIELD_SEPARATOR).collect::<Vec<_>>();
        let [boot, encoded_boot, pid, start, invocation] = fields.as_slice() else {
            return None;
        };
        let boot_known = match *boot {
            KNOWN_BOOT => true,
            UNKNOWN_BOOT => false,
            _ => return None,
        };
        let boot =
            crate::process::boot_identity::BootIdentity::from_encoded(boot_known, encoded_boot)?;
        let invocation_id = Uuid::parse_str(invocation).ok()?;
        if invocation_id.hyphenated().to_string() != *invocation {
            return None;
        }
        let pid = pid.parse().ok()?;
        let start_time = start.parse().ok()?;
        if pid == u32::default() || start_time == u64::default() {
            return None;
        }
        let owner = Self {
            boot,
            pid,
            start_time,
            invocation_id,
        };
        (owner.file_name() == original).then_some(owner)
    }

    pub fn process_relation(&self) -> crate::process::ProcessIdentityRelation {
        let boot_relation = self.boot.relation_to_current();
        if boot_relation == crate::process::BootIdentityRelation::Different {
            return crate::process::ProcessIdentityRelation::Gone;
        }
        match crate::process::process_identity_relation(self.pid, self.start_time) {
            crate::process::ProcessIdentityRelation::Gone => {
                crate::process::ProcessIdentityRelation::Gone
            }
            crate::process::ProcessIdentityRelation::Unknown => {
                crate::process::ProcessIdentityRelation::Unknown
            }
            crate::process::ProcessIdentityRelation::Matching
                if boot_relation == crate::process::BootIdentityRelation::Same =>
            {
                crate::process::ProcessIdentityRelation::Matching
            }
            crate::process::ProcessIdentityRelation::Matching => {
                crate::process::ProcessIdentityRelation::Unknown
            }
        }
    }

    pub fn boot_relation(&self) -> crate::process::BootIdentityRelation {
        self.boot.relation_to_current()
    }
}

fn require_known_boot(value: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        crate::process::boot_identity_relation(value, value)
            == crate::process::BootIdentityRelation::Same,
        BOOT_IDENTITY_UNAVAILABLE
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_settings_artifact_tests.rs"]
mod tests;
