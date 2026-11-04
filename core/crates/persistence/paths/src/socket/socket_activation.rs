#[path = "../socket_activation/constants.rs"]
mod constants;
#[path = "../socket_activation/io.rs"]
mod io;
#[path = "../socket_activation/publication.rs"]
mod publication;
#[path = "../socket_activation/stage.rs"]
mod stage;
#[path = "namespace/socket_namespace_unbound_probe.rs"]
mod unbound_probe;
pub(crate) use unbound_probe::prove_unbound_refused;

use self::constants::*;
use self::io::*;
pub use self::publication::SocketPublication;
use self::stage::StageEntry;
use super::directory::PrivateDirectory;
use super::{socket_path, socket_stage};
use anyhow::{Context, Result, bail};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::net::{UnixListener, UnixStream};

pub async fn publish(
    directory: PrivateDirectory,
    path: &Path,
    probe_timeout: Duration,
    live_error: &str,
) -> Result<(UnixListener, SocketPublication)> {
    publish_after(directory, path, probe_timeout, live_error, || Ok(())).await
}

pub async fn publish_after<F>(
    directory: PrivateDirectory,
    path: &Path,
    probe_timeout: Duration,
    live_error: &str,
    before_publication: F,
) -> Result<(UnixListener, SocketPublication)>
where
    F: FnOnce() -> Result<()>,
{
    let target = SocketTarget::new(directory, path)?;
    if let Some(identity) = target.inspect(true)? {
        match probe(&target, identity, probe_timeout).await? {
            Probe::Live => bail!("{live_error}: {}", path.display()),
            Probe::Refused => target.remove(identity)?,
        }
    }
    install_with_hook(target, before_publication).await
}

async fn probe(
    target: &SocketTarget,
    identity: SocketIdentity,
    timeout: Duration,
) -> Result<Probe> {
    let alias = StageEntry::link(target, identity, socket_stage::Kind::Probe).await?;
    let outcome = match tokio::time::timeout(timeout, UnixStream::connect(alias.path())).await {
        Ok(Ok(_)) => Ok(Probe::Live),
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            Ok(Probe::Refused)
        }
        Ok(Err(error)) => Err(error).context(PROBE_ERROR),
        Err(_) => Err(anyhow::anyhow!(PROBE_TIMEOUT_ERROR)),
    };
    drop(alias);
    anyhow::ensure!(
        socket_matches_at(
            &target.directory,
            &target.name,
            true,
            identity,
            crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT,
        )?,
        PROBE_LINK_COUNT_ERROR
    );
    outcome
}

async fn install_with_hook<F>(
    target: SocketTarget,
    after_staging: F,
) -> Result<(UnixListener, SocketPublication)>
where
    F: FnOnce() -> Result<()>,
{
    let mut stage = StageEntry::vacant(&target, socket_stage::Kind::Bind).await?;
    let listener = UnixListener::bind(stage.path()).context(BIND_ERROR)?;
    let identity = stage.inspect(false)?.context(BIND_DISAPPEARED_ERROR)?;
    stage.chmod(identity)?;
    after_staging()?;
    rename_exclusive(&stage, &target)?;
    stage.disarm();
    let publication = SocketPublication::new(target.directory, target.name, identity);
    if let Err(error) = publication.verify_published() {
        return match publication.withdraw() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(cleanup.context(error)),
        };
    }
    Ok((listener, publication))
}

enum Probe {
    Live,
    Refused,
}

struct SocketTarget {
    directory: PrivateDirectory,
    name: CString,
    stage: socket_stage::Target,
}

impl SocketTarget {
    fn new(directory: PrivateDirectory, path: &Path) -> Result<Self> {
        anyhow::ensure!(
            path.parent() == Some(directory.path()),
            PARENT_MISMATCH_ERROR
        );
        socket_path::validate(path)?;
        socket_stage::validate_parent(directory.path()).context(STAGE_PATH_BUDGET_ERROR)?;
        let name = c_name(path)?;
        let stage = socket_stage::Target::new(directory.path(), &name)?;
        Ok(Self {
            directory,
            name,
            stage,
        })
    }

    fn inspect(&self, private: bool) -> Result<Option<SocketIdentity>> {
        inspect_at(&self.directory, &self.name, private)
    }

    fn remove(&self, identity: SocketIdentity) -> Result<()> {
        anyhow::ensure!(
            socket_matches_at(
                &self.directory,
                &self.name,
                true,
                identity,
                crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT,
            )?,
            TARGET_CHANGED_ERROR
        );
        unlink_at(&self.directory, &self.name)?;
        self.directory.sync()?;
        anyhow::ensure!(self.inspect(false)?.is_none(), TARGET_REMOVE_ERROR);
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/socket_activation/adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/socket_activation/instance_matrix_tests.rs"]
mod instance_matrix_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/socket_activation/io_branch_tests.rs"]
mod io_branch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/socket_activation/path_tests.rs"]
mod path_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/socket_activation/tests.rs"]
mod tests;
