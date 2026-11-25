use super::{
    super::{
        SOCKET_NAMESPACE_PROOF_ENDPOINT_LIMIT, active_endpoints, socket_path, socket_stage, stream,
    },
    constants::*,
};
use crate::paths::directory::{PrivateDirectory, entry_c_string};
use anyhow::{Result, ensure};
use std::ffi::CString;

pub(super) type Endpoints = Vec<(CString, active_endpoints::SocketIdentity)>;

pub(super) fn inspect(directory: &PrivateDirectory) -> Result<Endpoints> {
    directory.verify_path()?;
    let mut entries = stream::DirectoryStream::open(directory.descriptor())?;
    let mut sockets = Vec::new();
    while let Some(name) = entries.next_name()? {
        let name = entry_c_string(&name)?;
        // This one bookkeeping entry is verified and locked separately. Every
        // target lease or stage entry remains evidence of possible publication.
        if socket_stage::is_lifecycle(&name) {
            continue;
        }
        ensure!(
            sockets.len() < SOCKET_NAMESPACE_PROOF_ENDPOINT_LIMIT,
            UNPROVEN
        );
        ensure!(socket_path::is_endpoint_name(&name), UNPROVEN);
        let identity = active_endpoints::inspect_socket(directory, &name)?
            .ok_or_else(|| anyhow::anyhow!(UNPROVEN))?;
        sockets.push((name, identity));
    }
    sockets.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    directory.verify_path()?;
    Ok(sockets)
}
