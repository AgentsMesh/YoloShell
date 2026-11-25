use super::constants::*;
use super::direct_child::force_unreaped_direct_child;
use super::protocol::DiscoveryCapability;
use super::watchdog_cleanup;
use anyhow::{Result, anyhow};
use std::process::{Child, ExitStatus};
use std::time::Instant;
use yolo_paths::IsolatedTestSocketRoot;
use yolo_process::{CapturedProcessTree, ProcessDiscoveryCapability, ProcessIdentity};

pub(super) struct ChildCustody {
    child: Option<Child>,
    identity: ProcessIdentity,
}

pub(super) struct RootCustody(Option<IsolatedTestSocketRoot>);
pub(super) struct DiscoveryCustody {
    process: ProcessDiscoveryCapability,
    protocol: DiscoveryCapability,
}
pub(super) struct TreeCustody {
    tree: Option<CapturedProcessTree>,
    settlement_attempted: bool,
}

impl ChildCustody {
    pub(super) fn new(child: Child, identity: ProcessIdentity) -> Self {
        Self {
            child: Some(child),
            identity,
        }
    }

    pub(super) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        let status = self.child.as_mut().unwrap().try_wait()?;
        if status.is_some() {
            self.child.take();
        }
        Ok(status)
    }

    pub(super) fn settle(&mut self) -> Result<()> {
        if self.child.is_none() {
            return Ok(());
        }
        let mut failure = force_unreaped_direct_child(
            self.child.as_mut().unwrap(),
            Some(self.identity),
            CHILD_REAP_DETAIL,
        )
        .err();
        let deadline = Instant::now() + CHILD_FORCE_REAP_TIMEOUT;
        while Instant::now() < deadline {
            match self.try_wait() {
                Ok(Some(_)) => return failure.map_or(Ok(()), Err),
                Ok(None) => {}
                Err(error) if failure.is_none() => failure = Some(error),
                Err(_) => {}
            }
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
        Err(failure.unwrap_or_else(|| anyhow!(CHILD_REAP_DETAIL)))
    }
}

impl Drop for ChildCustody {
    fn drop(&mut self) {
        let _ = self.settle();
    }
}

impl RootCustody {
    pub(super) fn create() -> Result<Self> {
        Ok(Self(Some(IsolatedTestSocketRoot::create()?)))
    }

    pub(super) fn path(&self) -> &std::path::Path {
        self.0.as_ref().unwrap().path()
    }

    pub(super) fn adopt(path: &std::path::Path) -> Result<Self> {
        Ok(Self(Some(IsolatedTestSocketRoot::adopt(path)?)))
    }

    pub(super) fn disarm(&mut self) {
        self.0.take();
    }

    pub(super) fn finalize(&mut self) -> Result<()> {
        let result = self.0.as_mut().unwrap().finalize();
        if result.is_ok() {
            self.0.take();
        }
        result
    }
}

impl DiscoveryCustody {
    pub(super) fn adopt_inherited() -> Result<Self> {
        let process = ProcessDiscoveryCapability::adopt_inherited_manager()?;
        let protocol = DiscoveryCapability::parse(process.value())?;
        Ok(Self { process, protocol })
    }

    pub(super) fn protocol(&self) -> &DiscoveryCapability {
        &self.protocol
    }

    pub(super) fn reader_handle(&self) -> u64 {
        self.process.reader_handle()
    }

    pub(super) fn writer_handle(&self) -> u64 {
        self.process.writer_handle()
    }

    pub(super) fn into_verified(
        self,
        identity: ProcessIdentity,
        descriptor: u64,
    ) -> Result<ProcessDiscoveryCapability> {
        anyhow::ensure!(
            self.process.writer_handle() == descriptor,
            WATCHDOG_PROTOCOL_DETAIL
        );
        self.process.verify_writer_holder(identity)?;
        Ok(self.process)
    }
}

impl TreeCustody {
    pub(super) fn capture(
        identity: ProcessIdentity,
        discovery: ProcessDiscoveryCapability,
        origin: ProcessIdentity,
    ) -> Result<Self> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        #[cfg(target_os = "linux")]
        let tree = discovery.capture_process_tree_with_origin(identity, Some(origin))?;
        #[cfg(target_os = "macos")]
        let tree = discovery.capture_process_tree(identity)?;
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let tree =
            CapturedProcessTree::capture_with_discovery(identity, discovery.value().to_owned())?;
        Ok(Self {
            tree: Some(tree),
            settlement_attempted: false,
        })
    }

    pub(super) fn refresh(&mut self) -> Result<()> {
        self.tree.as_mut().unwrap().refresh_if_needed().map(|_| ())
    }

    #[cfg(feature = "adversarial-test-hooks")]
    pub(super) fn acknowledge_historical_witness(&self) -> Result<()> {
        super::fault::acknowledge_historical_witness(self.tree.as_ref().unwrap())
    }

    pub(super) fn settle(&mut self) -> Result<()> {
        anyhow::ensure!(!self.settlement_attempted, TREE_SETTLEMENT_REPEATED_DETAIL);
        self.settlement_attempted = true;
        let result = watchdog_cleanup::settle(self.tree.as_mut().unwrap());
        if result.is_ok() {
            self.tree.take();
        }
        result
    }
}

impl Drop for TreeCustody {
    fn drop(&mut self) {
        if self.tree.is_some() && !self.settlement_attempted {
            let _ = self.settle();
        }
    }
}
