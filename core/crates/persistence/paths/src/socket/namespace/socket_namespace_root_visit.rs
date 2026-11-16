use super::*;
use anyhow::Result;
use std::os::unix::ffi::OsStrExt;
#[path = "socket_namespace_stage_authority.rs"]
mod stage_authority;
pub(super) use stage_authority::StageAuthority;

pub(super) enum Work {
    Complete,
    Progress,
}

pub(super) enum Visit {
    Pending {
        revision: crate::paths::directory::DirectoryRevision,
        directory: PrivateDirectory,
        authority: StageAuthority,
    },
}

pub(super) fn visit(
    root: &PrivateDirectory,
    name: &std::ffi::OsStr,
    _lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
) -> Result<Option<Visit>> {
    if !is_namespace_name(name) {
        return Ok(None);
    }
    let path = root.path().join(name);
    let name_c = crate::paths::directory::entry_c_string(name)?;
    if let candidate::Candidate::Open(directory) = candidate::open(root, &path, &name_c)?
        && let Some(authority) = StageAuthority::capture(&directory, name)?
    {
        return Ok(Some(Visit::Pending {
            revision: root.revision()?,
            directory,
            authority,
        }));
    }
    Ok(None)
}

fn is_namespace_name(name: &std::ffi::OsStr) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == SOCKET_HASH_HEX_LENGTH
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}
