use super::{HOST_INSTANCE_LOCK_COUNT, HOST_INSTANCE_LOCK_MAX_BYTES, Host};
use crate::coordination::{CoordinationLock, CoordinationScope};
use crate::paths::{Paths, directory::PrivateDirectory};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

const ABSENCE_CHANGED: &str = "Host absence evidence changed";
const SOCKET_PARENT_MISSING: &str = "Host socket path has no parent";
const NAMESPACE_ROOT_MISSING: &str = "Host socket namespace has no root";

pub struct HostAbsence {
    locks: [CoordinationLock; HOST_INSTANCE_LOCK_COUNT],
    paths: Paths,
    home: Option<PrivateDirectory>,
    directories: Vec<ObservedDirectory>,
    home_lock_path: PathBuf,
    endpoint_lock_path: PathBuf,
    socket_override: Option<PathBuf>,
    default_socket: PathBuf,
    socket: PathBuf,
}

struct ObservedDirectory {
    path: PathBuf,
    retained: Option<PrivateDirectory>,
}

impl Host {
    pub fn observe_absence(home: &Path, socket: Option<&Path>) -> Result<Option<HostAbsence>> {
        crate::paths::validate_home(home)?;
        let paths = Paths::new(home);
        let home_lock_path =
            crate::coordination::lock_path(CoordinationScope::HostHome, home, None)?;
        let Some(home_lock) =
            CoordinationLock::try_acquire(&home_lock_path, HOST_INSTANCE_LOCK_MAX_BYTES)?
        else {
            return Ok(None);
        };
        let home = open_directory(home)?;
        let default_socket = paths.host_socket()?;
        let namespace_directory = default_socket.parent().context(SOCKET_PARENT_MISSING)?;
        let namespace_root = namespace_directory
            .parent()
            .context(NAMESPACE_ROOT_MISSING)?;
        let socket_override = socket.map(Path::to_path_buf);
        let socket = socket
            .map(Path::to_path_buf)
            .unwrap_or_else(|| default_socket.clone());
        super::validate_socket_namespace_selection(&socket, &default_socket, namespace_root)?;
        super::ensure_socket_is_not_lock(&socket, &home_lock_path)?;
        super::validate_socket_path(&socket)?;
        let endpoint =
            crate::coordination::lock_path(CoordinationScope::HostEndpoint, &socket, None)?;
        let Some(endpoint_lock) =
            CoordinationLock::try_acquire(&endpoint, HOST_INSTANCE_LOCK_MAX_BYTES)?
        else {
            return Ok(None);
        };
        if !missing_socket(&socket)? {
            return Ok(None);
        }
        let directories = [
            namespace_root,
            namespace_directory,
            socket.parent().context(SOCKET_PARENT_MISSING)?,
        ]
        .into_iter()
        .map(ObservedDirectory::capture)
        .collect::<Result<_>>()?;
        let absence = HostAbsence {
            locks: [home_lock, endpoint_lock],
            paths,
            home,
            directories,
            home_lock_path,
            endpoint_lock_path: endpoint,
            socket_override,
            default_socket,
            socket,
        };
        absence.verify()?;
        Ok(Some(absence))
    }
}

impl HostAbsence {
    pub fn verify(&self) -> Result<()> {
        for lock in &self.locks {
            lock.verify()?;
        }
        ensure!(
            crate::coordination::lock_path(CoordinationScope::HostHome, self.paths.home(), None)?
                == self.home_lock_path,
            ABSENCE_CHANGED
        );
        match &self.home {
            Some(home) => home.verify_path()?,
            None => ensure!(
                open_directory(self.paths.home())?.is_none(),
                ABSENCE_CHANGED
            ),
        }
        for directory in &self.directories {
            directory.verify()?;
        }
        let default_socket = self.paths.host_socket()?;
        ensure!(default_socket == self.default_socket, ABSENCE_CHANGED);
        let socket = match &self.socket_override {
            Some(socket) => socket.clone(),
            None => default_socket,
        };
        ensure!(socket == self.socket, ABSENCE_CHANGED);
        ensure!(
            crate::coordination::lock_path(CoordinationScope::HostEndpoint, &socket, None)?
                == self.endpoint_lock_path,
            ABSENCE_CHANGED
        );
        ensure!(missing_socket(&self.socket)?, ABSENCE_CHANGED);
        for lock in &self.locks {
            lock.verify()?;
        }
        Ok(())
    }
}

impl ObservedDirectory {
    fn capture(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            retained: open_directory(path)?,
        })
    }

    fn verify(&self) -> Result<()> {
        match &self.retained {
            Some(directory) => directory.verify_path(),
            None => {
                ensure!(open_directory(&self.path)?.is_none(), ABSENCE_CHANGED);
                Ok(())
            }
        }
    }
}

fn open_directory(path: &Path) -> Result<Option<PrivateDirectory>> {
    match PrivateDirectory::open(path) {
        Ok(directory) => Ok(Some(directory)),
        Err(error)
            if crate::paths::private_file::io_kind(&error)
                == Some(std::io::ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn missing_socket(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/host/existing_presence.rs"]
mod tests;
