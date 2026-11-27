use crate::process::linux_containment::{LinuxContainmentBroker, LinuxContainmentLease};
use crate::process::{ProcessIdentity, PtySessionProof};
use crate::worker::{Paths, StartupChildGuard, WorkerNamespace, persist_startup_containment};
use anyhow::{Context, Result};
use uuid::Uuid;

const DELEGATION_REQUIRED: &str = "Linux PTY fixture requires delegated cgroup containment";
const EMPTY_CGROUP_CLEANUP_DETAIL: &str = "remove unbound Linux PTY fixture cgroup";

pub(in crate::worker) struct FixtureContainment {
    lease: Option<LinuxContainmentLease>,
    paths: Paths,
    namespace: WorkerNamespace,
}

impl FixtureContainment {
    pub(in crate::worker) fn prepare(
        paths: Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        proof: &PtySessionProof,
    ) -> Result<Self> {
        paths.ensure_session(terminal_id)?;
        let namespace = WorkerNamespace::capture(&paths, terminal_id)?;
        let mut lease = LinuxContainmentBroker::discover()?
            .context(DELEGATION_REQUIRED)?
            .prepare(terminal_id, runtime_epoch)?;
        if let Err(error) = lease
            .bind_pty_proof(proof)
            .map_err(anyhow::Error::from)
            .and_then(|()| persist_startup_containment(&namespace, &paths, lease.record()))
        {
            lease.remove_empty()?;
            return Err(error);
        }
        Ok(Self {
            lease: Some(lease),
            paths,
            namespace,
        })
    }

    pub(in crate::worker) fn bind(
        mut self,
        guard: &mut StartupChildGuard,
        root: ProcessIdentity,
    ) -> Result<()> {
        let record = guard.bind_linux_containment(self.lease.take().unwrap(), root)?;
        persist_startup_containment(&self.namespace, &self.paths, &record)
    }
}

impl Drop for FixtureContainment {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take()
            && let Err(error) = lease.remove_empty()
        {
            tracing::warn!(%error, EMPTY_CGROUP_CLEANUP_DETAIL);
        }
    }
}
