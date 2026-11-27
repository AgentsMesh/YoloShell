use super::{constants::*, persisted_probe::Probe, topology::Location};
use crate::hub::federation_support::{POLL, deadline};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_observation_store::OBSERVATION_DIRECTORY;
use yolo_runtime::paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub async fn wait(location: &Location, origins: &[PeerIdentity], agents: &[AgentSnapshot]) {
    let owner = location.peer;
    let path = location.state.clone();
    let origins = origins
        .iter()
        .map(|peer| (peer.hub(), *peer))
        .collect::<BTreeMap<_, _>>();
    let expected = agents
        .iter()
        .filter(|agent| agent.cursor().target().address.hub_id != owner.hub())
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(origins.len(), EXPECTED_REMOTE_AGENTS);
    assert_eq!(expected.len(), EXPECTED_REMOTE_AGENTS);
    let limit = deadline();
    let canceled = Arc::new(AtomicBool::new(false));
    let _cancel = Cancel(canceled.clone());
    let (done, result) = tokio::sync::oneshot::channel();
    // Keep filesystem I/O off the Tokio worker. At most one read-only probe
    // thread exists; a stuck syscall cannot prevent the test's deadline firing.
    std::thread::Builder::new()
        .name(CACHE_PROBE_THREAD.into())
        .spawn(move || {
            let read = || -> Result<()> {
                let root = PrivateDirectory::open(&path)?;
                let directory = root.open_descendant(&root.path().join(OBSERVATION_DIRECTORY))?;
                let identity = directory.durable_identity()?;
                let path = directory.path().to_path_buf();
                let namespace = NamespaceLease::from_directories(vec![
                    (CACHE_ROOT_LABEL, root),
                    (CACHE_DIRECTORY_LABEL, directory),
                ])?;
                let probe = Probe {
                    namespace,
                    directory: path,
                    identity,
                    owner,
                    origins,
                    expected,
                };
                loop {
                    ensure!(
                        std::time::Instant::now() < limit.into_std()
                            && !canceled.load(Ordering::Acquire),
                        CACHE_PROBE_TIMEOUT
                    );
                    let published = probe.published()?;
                    ensure!(
                        std::time::Instant::now() < limit.into_std()
                            && !canceled.load(Ordering::Acquire),
                        CACHE_PROBE_TIMEOUT
                    );
                    if published {
                        return Ok(());
                    }
                    std::thread::sleep(POLL);
                }
            };
            let _ = done.send(read());
        })
        .unwrap();
    tokio::time::timeout_at(limit, result)
        .await
        .expect(CACHE_PROBE_TIMEOUT)
        .expect("cache evidence thread exited without a result")
        .expect("invalid published observation cache");
}

struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
