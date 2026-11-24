use super::constants::*;
use super::socket_bind_at;
use super::socket_directory::SocketDirectory;
use super::socket_identity::SocketIdentity;
use anyhow::{Context, Result, ensure};
use std::ffi::{CStr, CString};
use std::io::ErrorKind;
use tokio::net::UnixListener;
use uuid::Uuid;

const COLLISION_ERRORS: &[ErrorKind] = &[ErrorKind::AlreadyExists, ErrorKind::AddrInUse];

pub(super) struct BoundSocket {
    pub(super) listener: UnixListener,
    pub(super) cleanup: SocketCleanup,
}

pub(super) struct SocketCleanup {
    directory: SocketDirectory,
    name: CString,
    identity: SocketIdentity,
}

impl SocketCleanup {
    fn new(directory: SocketDirectory, name: CString, identity: SocketIdentity) -> Self {
        Self {
            directory,
            name,
            identity,
        }
    }
}

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = self.directory.remove_expected(&self.name, self.identity);
    }
}

pub(super) fn prepare_endpoint(directory: &SocketDirectory, name: &CStr) -> Result<()> {
    if let Some(identity) = directory.identity(name)? {
        directory.remove_expected(name, identity)?;
    }
    Ok(())
}

pub(super) fn bind_private(directory: &SocketDirectory, target: CString) -> Result<BoundSocket> {
    bind_private_with(directory, target, || Ok(()), |_, _| {})
}

fn bind_private_with<F, H>(
    directory: &SocketDirectory,
    target: CString,
    before_bind: F,
    after_bind: H,
) -> Result<BoundSocket>
where
    F: FnOnce() -> Result<()>,
    H: FnOnce(&CStr, &mut socket_bind_at::RelativeBinding),
{
    bind_private_using(directory, target, before_bind, after_bind, staging_name)
}

fn bind_private_using<F, H, N>(
    directory: &SocketDirectory,
    target: CString,
    before_bind: F,
    after_bind: H,
    next_name: N,
) -> Result<BoundSocket>
where
    F: FnOnce() -> Result<()>,
    H: FnOnce(&CStr, &mut socket_bind_at::RelativeBinding),
    N: FnMut() -> Result<CString>,
{
    let mut staging = StagedSocket::bind(directory, before_bind, after_bind, next_name)?;
    directory.publish_expected_from(directory, &staging.name, &target, staging.identity)?;
    ensure!(
        directory.identity(&target)? == Some(staging.identity),
        BROKER_SOCKET_PUBLISH_CHANGED
    );
    staging.armed = false;
    let listener = staging.listener.take();
    let listener = listener.context(BROKER_SOCKET_STAGING_LISTENER_MISSING)?;
    let cleanup = SocketCleanup::new(directory.try_clone()?, target, staging.identity);
    Ok(BoundSocket { listener, cleanup })
}

struct StagedSocket {
    listener: Option<UnixListener>,
    directory: SocketDirectory,
    name: CString,
    identity: SocketIdentity,
    armed: bool,
}

impl StagedSocket {
    fn bind<F, H, N>(
        directory: &SocketDirectory,
        before_bind: F,
        after_bind: H,
        mut next_name: N,
    ) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
        H: FnOnce(&CStr, &mut socket_bind_at::RelativeBinding),
        N: FnMut() -> Result<CString>,
    {
        let (mut before_bind, mut after_bind) = (Some(before_bind), Some(after_bind));
        for _ in 0..BROKER_SOCKET_STAGING_ATTEMPTS {
            let name = next_name()?;
            let retained = directory.try_clone()?;
            directory.verify_path()?;
            if let Some(hook) = before_bind.take() {
                hook()?;
            }
            let mut binding =
                match socket_bind_at::bind(directory.descriptor(), directory.path(), &name) {
                    Ok(value) => value,
                    Err(error) if COLLISION_ERRORS.contains(&error.kind()) => continue,
                    Err(error) => return Err(error).context(BROKER_SOCKET_STAGING_BIND_CONTEXT),
                };
            let identity = binding.identity;
            let mut staged = Self {
                listener: None,
                directory: retained,
                name,
                identity,
                armed: true,
            };
            if let Some(hook) = after_bind.take() {
                hook(&staged.name, &mut binding);
            }
            directory.set_permissions(&staged.name, identity)?;
            directory.verify_path()?;
            if let Some(error) = binding.restore_error {
                return Err(error).context(BROKER_SOCKET_STAGING_BIND_CONTEXT);
            }
            binding
                .listener
                .set_nonblocking(true)
                .context(BROKER_SOCKET_STAGING_NONBLOCKING_CONTEXT)?;
            staged.listener = Some(
                UnixListener::from_std(binding.listener)
                    .context(BROKER_SOCKET_STAGING_REGISTER_CONTEXT)?,
            );
            return Ok(staged);
        }
        anyhow::bail!(BROKER_SOCKET_STAGING_EXHAUSTED)
    }
}

#[cfg(test)]
pub(super) fn bind_private_with_hook<F>(
    directory: &SocketDirectory,
    target: CString,
    before_bind: F,
) -> Result<BoundSocket>
where
    F: FnOnce() -> Result<()>,
{
    bind_private_with(directory, target, before_bind, |_, _| {})
}

#[cfg(test)]
pub(super) fn bind_private_with_restore_failure(
    directory: &SocketDirectory,
    target: CString,
    message: &'static str,
) -> Result<BoundSocket> {
    bind_private_with(
        directory,
        target,
        || Ok(()),
        |_, binding| {
            binding.inject_restore_failure(message);
        },
    )
}

impl Drop for StagedSocket {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.directory.remove_expected(&self.name, self.identity);
        }
    }
}

fn staging_name() -> Result<CString> {
    CString::new(format!("{BROKER_SOCKET_STAGING_PREFIX}{}", Uuid::new_v4()))
        .context(BROKER_SOCKET_STAGING_NAME_CONTEXT)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_binding_instance_matrix.rs"]
mod instance_matrix;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_binding_tests.rs"]
mod tests;
