use super::{LinuxContainmentError, LinuxContainmentRecord, constants, descriptor, io};
use crate::process::ProcessIdentity;
use std::fs::File;

const RECORD_DETAIL: &str = "Linux settlement requires complete containment evidence";
const ROOT_DETAIL: &str = "Linux settlement delegated root identity changed";
const LEAF_DETAIL: &str = "Linux settlement leaf identity changed";
const RETIREMENT_DETAIL: &str = "Linux settlement retained leaf is not retired";

/// Read-only evidence for one completed containment recovery.
///
/// An already-absent leaf retains the existing same-root recovery contract; it
/// does not assert that a retained kernel descriptor witnessed leaf deletion.
pub struct LinuxContainmentSettlement {
    record: LinuxContainmentRecord,
    root: File,
    bound_root: ProcessIdentity,
    source: SettlementSource,
}

enum SettlementSource {
    RetainedLeaf { leaf: File, events: File },
    AlreadyAbsent,
}

impl LinuxContainmentSettlement {
    pub fn record(&self) -> &LinuxContainmentRecord {
        &self.record
    }

    /// Observe durable recovery evidence without freezing, signaling or removing.
    /// A matching leaf that remains present is pending, including an empty leaf.
    pub fn observe_settled(
        record: LinuxContainmentRecord,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        bound_root: ProcessIdentity,
    ) -> Result<Option<Self>, LinuxContainmentError> {
        require_complete_record(&record, terminal_id, runtime_epoch, bound_root)?;
        let root = open_configured_root(&record)?;
        match descriptor::open_leaf(&root, &record.leaf_name) {
            Ok(leaf) => {
                require_leaf_identity(&leaf, &record)?;
                verify_root(&root, &record)?;
                return Ok(None);
            }
            Err(LinuxContainmentError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let witness = Self {
            record,
            root,
            bound_root,
            source: SettlementSource::AlreadyAbsent,
        };
        witness.verify()?;
        Ok(Some(witness))
    }

    pub fn verify(&self) -> Result<(), LinuxContainmentError> {
        require_complete_record(
            &self.record,
            self.record.terminal_id,
            self.record.runtime_epoch,
            self.bound_root,
        )?;
        verify_root(&self.root, &self.record)?;
        if let SettlementSource::RetainedLeaf { leaf, events } = &self.source {
            require_leaf_identity(leaf, &self.record)?;
            if !io::retained_control_was_removed(events)? {
                return Err(LinuxContainmentError::Identity(RETIREMENT_DETAIL));
            }
        }
        match descriptor::open_leaf(&self.root, &self.record.leaf_name) {
            Err(LinuxContainmentError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => return Err(LinuxContainmentError::Identity(LEAF_DETAIL)),
            Err(error) => return Err(error),
        }
        verify_root(&self.root, &self.record)
    }

    // Recovery retains these handles before settlement can remove the leaf.
    // Only into_settlement exposes the candidate, after completion and verify.
    pub(super) fn capture(
        root: &File,
        leaf: Option<&File>,
        record: LinuxContainmentRecord,
        bound_root: ProcessIdentity,
    ) -> Result<Self, LinuxContainmentError> {
        let source = match leaf {
            Some(leaf) => SettlementSource::RetainedLeaf {
                leaf: leaf.try_clone()?,
                events: io::open_control(leaf, constants::CGROUP_EVENTS_FILE, libc::O_RDONLY)?,
            },
            None => SettlementSource::AlreadyAbsent,
        };
        Ok(Self {
            record,
            root: root.try_clone()?,
            bound_root,
            source,
        })
    }
}

fn require_complete_record(
    record: &LinuxContainmentRecord,
    terminal_id: uuid::Uuid,
    runtime_epoch: uuid::Uuid,
    bound_root: ProcessIdentity,
) -> Result<(), LinuxContainmentError> {
    if !record.validate_recovery(
        terminal_id,
        runtime_epoch,
        &super::super::boot_id(),
        bound_root,
    )? || record.root_identity.mount_id != record.leaf_identity.mount_id
    {
        return Err(LinuxContainmentError::Identity(RECORD_DETAIL));
    }
    Ok(())
}

fn open_configured_root(record: &LinuxContainmentRecord) -> Result<File, LinuxContainmentError> {
    if std::env::var_os(constants::DELEGATED_ROOT_ENVIRONMENT).as_deref()
        != Some(record.delegated_root.as_os_str())
    {
        return Err(LinuxContainmentError::Identity(ROOT_DETAIL));
    }
    let root = descriptor::open_root(&record.delegated_root)?;
    if descriptor::identity(&root)? != record.root_identity {
        return Err(LinuxContainmentError::Identity(ROOT_DETAIL));
    }
    Ok(root)
}

fn verify_root(root: &File, record: &LinuxContainmentRecord) -> Result<(), LinuxContainmentError> {
    if descriptor::identity(root)? != record.root_identity {
        return Err(LinuxContainmentError::Identity(ROOT_DETAIL));
    }
    open_configured_root(record).map(|_| ())
}

fn require_leaf_identity(
    leaf: &File,
    record: &LinuxContainmentRecord,
) -> Result<(), LinuxContainmentError> {
    if descriptor::identity(leaf)? != record.leaf_identity {
        return Err(LinuxContainmentError::Identity(LEAF_DETAIL));
    }
    Ok(())
}
