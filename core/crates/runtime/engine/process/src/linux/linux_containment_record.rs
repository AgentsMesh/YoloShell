use super::constants::RECORD_SCHEMA_VERSION;
use super::{LinuxContainmentError, LinuxContainmentRecord};
use uuid::Uuid;

impl LinuxContainmentRecord {
    pub fn validate(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        current_boot_id: &str,
    ) -> Result<(), LinuxContainmentError> {
        if self.schema_version != RECORD_SCHEMA_VERSION {
            return Err(LinuxContainmentError::Identity(
                "unsupported containment record schema",
            ));
        }
        if self.terminal_id != terminal_id || self.runtime_epoch != runtime_epoch {
            return Err(LinuxContainmentError::Identity(
                "containment runtime identity mismatch",
            ));
        }
        if self.leaf_name != super::constants::session_leaf_name(terminal_id, runtime_epoch) {
            return Err(LinuxContainmentError::Identity(
                "containment leaf does not match its runtime identity",
            ));
        }
        if self.boot_id != current_boot_id
            || self
                .boot_id
                .starts_with(super::super::constants::UNKNOWN_BOOT_ID_PREFIX)
        {
            return Err(LinuxContainmentError::Identity(
                "containment belongs to another or unknown boot",
            ));
        }
        if self.lease_token.is_nil() {
            return Err(LinuxContainmentError::Identity(
                "containment lease token is invalid",
            ));
        }
        if self.pty_device.is_some() != self.pty_capability.is_some()
            || self.bound_pid.is_some() != self.bound_start_time.is_some()
            || self.pty_device == Some(u64::default())
            || self.pty_capability.as_ref().is_some_and(String::is_empty)
            || self.bound_pid == Some(u32::default())
            || self.bound_start_time == Some(u64::default())
        {
            return Err(LinuxContainmentError::Identity(
                "containment custody evidence is inconsistent",
            ));
        }
        super::descriptor::validate_leaf_name(&self.leaf_name)
    }

    pub fn validate_recovery(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        current_boot_id: &str,
        root: super::super::ProcessIdentity,
    ) -> Result<bool, LinuxContainmentError> {
        self.validate(terminal_id, runtime_epoch, current_boot_id)?;
        let Some((pid, start_time)) = self.bound_pid.zip(self.bound_start_time) else {
            return Ok(false);
        };
        if self.pty_device.is_none()
            || self.pty_capability.is_none()
            || pid != root.pid
            || start_time != root.start_time
        {
            return Err(LinuxContainmentError::Identity(
                "containment recovery evidence is incomplete or mismatched",
            ));
        }
        Ok(true)
    }
}
