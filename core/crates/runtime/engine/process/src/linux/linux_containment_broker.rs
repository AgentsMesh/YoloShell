use super::constants::*;
use super::descriptor::{create_leaf, identity, open_leaf, open_root, remove_exact_leaf};
use super::io::{contains_word, write_control};
use super::{LinuxContainmentBroker, LinuxContainmentError, LinuxContainmentLease};
use std::path::PathBuf;
use uuid::Uuid;

impl LinuxContainmentBroker {
    pub fn discover() -> Result<Option<Self>, LinuxContainmentError> {
        let Some(value) = std::env::var_os(DELEGATED_ROOT_ENVIRONMENT) else {
            return Ok(None);
        };
        let root_path = PathBuf::from(value);
        let root = open_root(&root_path)?;
        let root_identity = identity(&root)?;
        if !contains_word(&root, CGROUP_CONTROLLERS_FILE, PIDS_CONTROLLER)? {
            return Err(LinuxContainmentError::Unavailable(
                "delegation does not expose the pids controller",
            ));
        }
        if !contains_word(&root, CGROUP_SUBTREE_CONTROL_FILE, PIDS_CONTROLLER)? {
            write_control(&root, CGROUP_SUBTREE_CONTROL_FILE, PIDS_ENABLE_COMMAND)?;
        }
        Ok(Some(Self {
            root,
            root_path,
            root_identity,
        }))
    }

    pub fn prepare(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<LinuxContainmentLease, LinuxContainmentError> {
        let leaf_name = session_leaf_name(terminal_id, runtime_epoch);
        let leaf = match create_leaf(&self.root, &leaf_name) {
            Ok(leaf) => leaf,
            Err(LinuxContainmentError::Io(error))
                if error.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                return Err(LinuxContainmentError::Unavailable(
                    "session cgroup already exists",
                ));
            }
            Err(error) => return Err(error),
        };
        let cleanup_leaf = match leaf.try_clone() {
            Ok(cleanup) => cleanup,
            Err(error) => {
                let _ = remove_exact_leaf(&self.root, &leaf, &leaf_name);
                return Err(error.into());
            }
        };
        let prepared = (|| {
            write_control(&leaf, PIDS_MAX_FILE, PIDS_MAX_VALUE)?;
            require_leaf_controls(&leaf)?;
            let record = super::LinuxContainmentRecord {
                schema_version: RECORD_SCHEMA_VERSION,
                terminal_id,
                runtime_epoch,
                boot_id: super::super::boot_id(),
                delegated_root: self.root_path.clone(),
                root_identity: self.root_identity,
                leaf_name: leaf_name.clone(),
                leaf_identity: identity(&leaf)?,
                lease_token: Uuid::new_v4(),
                pty_device: None,
                pty_capability: None,
                bound_pid: None,
                bound_start_time: None,
            };
            record.validate(terminal_id, runtime_epoch, &record.boot_id)?;
            Ok(LinuxContainmentLease {
                root: self.root.try_clone()?,
                leaf,
                record,
                bound_root: None,
                freeze_requested_at: None,
                membership_unstable_since: None,
            })
        })();
        if prepared.is_err() {
            let _ = remove_exact_leaf(&self.root, &cleanup_leaf, &leaf_name);
        }
        prepared
    }

    pub(super) fn reopen_recovery(
        &self,
        record: super::LinuxContainmentRecord,
    ) -> Result<Option<LinuxContainmentLease>, LinuxContainmentError> {
        record.validate(
            record.terminal_id,
            record.runtime_epoch,
            &super::super::boot_id(),
        )?;
        if record.delegated_root != self.root_path || record.root_identity != self.root_identity {
            return Err(LinuxContainmentError::Identity(
                "delegated cgroup root identity changed",
            ));
        }
        let leaf = match open_leaf(&self.root, &record.leaf_name) {
            Ok(leaf) => leaf,
            Err(LinuxContainmentError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if identity(&leaf)? != record.leaf_identity {
            return Err(LinuxContainmentError::Identity(
                "session cgroup leaf identity changed",
            ));
        }
        require_leaf_controls(&leaf)?;
        let bound_root = record
            .bound_pid
            .zip(record.bound_start_time)
            .map(|(pid, start_time)| super::super::ProcessIdentity { pid, start_time });
        Ok(Some(LinuxContainmentLease {
            root: self.root.try_clone()?,
            leaf,
            record,
            bound_root,
            freeze_requested_at: None,
            membership_unstable_since: None,
        }))
    }
}

fn require_leaf_controls(leaf: &std::fs::File) -> Result<(), LinuxContainmentError> {
    for name in [
        CGROUP_EVENTS_FILE,
        CGROUP_FREEZE_FILE,
        CGROUP_PROCESSES_FILE,
        PIDS_MAX_FILE,
    ] {
        super::io::read_control(leaf, name)?;
    }
    Ok(())
}
