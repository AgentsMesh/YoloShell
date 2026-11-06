use super::*;
use std::sync::Mutex;
mod bridge_rpc;
mod child_guard;
#[path = "cleanup/cleanup.rs"]
mod cleanup;
#[cfg(test)]
#[path = "cleanup/cleanup_authority_tests.rs"]
mod cleanup_authority_tests;
#[cfg(test)]
#[path = "cleanup/cleanup_binding_tests.rs"]
mod cleanup_binding_tests;
#[path = "cleanup/cleanup_connect.rs"]
mod cleanup_connect;
#[path = "cleanup/cleanup_connect_native.rs"]
mod cleanup_connect_native;
#[cfg(test)]
#[path = "cleanup/cleanup_connect_peer_tests.rs"]
mod cleanup_connect_peer_tests;
#[cfg(test)]
#[path = "cleanup/cleanup_connect_tests.rs"]
mod cleanup_connect_tests;
#[path = "cleanup/cleanup_issues.rs"]
mod cleanup_issues;
#[cfg(test)]
#[path = "cleanup/cleanup_status_authority_tests.rs"]
mod cleanup_status_authority_tests;
#[cfg(all(test, target_os = "linux"))]
#[path = "cleanup/cleanup_successor_linux_tests.rs"]
mod cleanup_successor_tests;
#[cfg(all(test, not(target_os = "linux")))]
#[path = "cleanup/cleanup_successor_tests.rs"]
mod cleanup_successor_tests;
#[path = "cleanup/cleanup_tree.rs"]
mod cleanup_tree;
#[cfg(test)]
#[path = "cleanup/cleanup_tree_tests.rs"]
mod cleanup_tree_tests;
#[path = "cleanup/cleanup_wait.rs"]
mod cleanup_wait;
mod constants;
#[cfg(test)]
mod constants_tests;
#[path = "harness/harness_cleanup.rs"]
mod harness_cleanup;
#[path = "harness/harness_creation.rs"]
mod harness_creation;
#[path = "harness/harness_creation_readiness.rs"]
mod harness_creation_readiness;
#[cfg(test)]
#[path = "harness/harness_creation_readiness_tests.rs"]
mod harness_creation_readiness_tests;
#[path = "harness/harness_host_replacement.rs"]
mod harness_host_replacement;
#[path = "harness/harness_namespace_retirement.rs"]
mod harness_namespace_retirement;
#[path = "harness/harness_runtime_tracking.rs"]
mod harness_runtime_tracking;
#[path = "host/host_adoption.rs"]
mod host_adoption;
#[cfg(test)]
#[path = "host/host_diagnostics_tests.rs"]
mod host_diagnostics_tests;
#[path = "host/host_guard.rs"]
mod host_guard;
#[path = "host/host_log.rs"]
mod host_log;
#[path = "host/host_process.rs"]
mod host_process;
#[path = "host/host_readiness.rs"]
mod host_readiness;
#[path = "power_loss/power_loss.rs"]
mod power_loss;
#[path = "power_loss/power_loss_identity.rs"]
mod power_loss_identity;
mod process;
mod relay_status;
#[path = "runtime_record/runtime_record_authority.rs"]
mod runtime_record_authority;
#[cfg(test)]
#[path = "runtime_record/runtime_record_refresh_tests.rs"]
mod runtime_record_refresh_tests;
#[cfg(test)]
#[path = "runtime_record/runtime_record_remote_boot_tests.rs"]
mod runtime_record_remote_boot_tests;
#[cfg(test)]
#[path = "runtime_record/runtime_record_test_support.rs"]
mod runtime_record_test_support;
#[path = "runtime_record/runtime_record_validation.rs"]
mod runtime_record_validation;
#[cfg(test)]
#[path = "runtime_record/runtime_record_validation_tests.rs"]
mod runtime_record_validation_tests;
mod wire;

pub(crate) use bridge_rpc::*;
pub(crate) use child_guard::*;
pub(crate) use cleanup::*;
pub(crate) use cleanup_wait::*;
pub(crate) use constants::*;
pub(crate) use host_guard::*;
pub(crate) use host_process::*;
pub(crate) use host_readiness::*;
pub(crate) use power_loss::*;
pub(crate) use power_loss_identity::*;
pub(crate) use process::*;
pub(crate) use relay_status::*;
pub(crate) use wire::*;
pub(crate) struct Harness {
    pub(crate) home: TempDir,
    host: HostGuard,
    runtimes: Mutex<Vec<runtime_record_authority::BoundRuntimeRecord>>,
    cleanup_trees: Mutex<cleanup_tree::RuntimeProcessTrees>,
    creations: Mutex<std::collections::HashMap<Uuid, (Uuid, Uuid)>>,
    namespace_retirement: Option<harness_namespace_retirement::NamespaceRetirement>,
}

impl Harness {
    pub(crate) async fn start() -> Self {
        let home = tempfile::tempdir().unwrap();
        let host = spawn_host(home.path());
        Self::start_with_host(home, host).await
    }

    pub(crate) async fn start_with_host(home: TempDir, host: SpawnedHost) -> Self {
        let host = HostGuard::new(host);
        let mut host = host;
        wait_for_host(home.path(), &mut host).await;
        Self {
            home,
            host,
            runtimes: Mutex::new(Vec::new()),
            cleanup_trees: Mutex::new(cleanup_tree::RuntimeProcessTrees::default()),
            creations: Mutex::new(std::collections::HashMap::new()),
            namespace_retirement: None,
        }
    }

    pub(crate) fn paths(&self) -> Paths {
        Paths::new(self.home.path())
    }

    pub(crate) fn client(&self) -> Client {
        Client::new(self.paths().host_socket().unwrap())
    }

    pub(crate) fn host_stderr_tail(&self) -> String {
        self.host.stderr_tail()
    }

    pub(crate) fn host_identity(&self) -> ProcessIdentity {
        self.host.identity()
    }

    pub(crate) fn try_wait_host(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.host.try_wait()
    }

    pub(crate) fn kill_host_hard(&mut self) {
        self.host.kill_hard();
    }

    pub(crate) fn stop_host_with_sigterm(&mut self) {
        self.host.stop_with_sigterm();
    }
}
