use super::remount_protocol::{self, PHASE_ENV, ROOT_ENV};
use std::os::unix::fs::PermissionsExt;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

#[test]
fn durable_migration_remount_handoff() -> anyhow::Result<()> {
    let root = std::env::var_os(ROOT_ENV);
    let phase = std::env::var(PHASE_ENV).ok();
    if root.is_none() && phase.is_none() {
        return Ok(());
    }
    let root = root.ok_or_else(|| anyhow::anyhow!(ROOT_ENV))?;
    let phase = phase.ok_or_else(|| anyhow::anyhow!(PHASE_ENV))?;
    let phase = serde_json::from_value(serde_json::Value::String(phase))?;
    remount_protocol::run(std::path::Path::new(&root), phase)
}

#[test]
fn migration_reopens_every_durable_boundary_with_a_fresh_owner() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        root.path(),
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    for phase in remount_protocol::PHASES {
        remount_protocol::run(root.path(), phase).unwrap();
    }
}
