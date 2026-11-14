use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_assets::IngressTestAssets;
use super::support::Fixture;
use crate::paths::{IsolatedTestSocketRoot, Paths};
use crate::worker::WorkerNamespace;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

#[path = "ingress_test_registration.rs"]
mod registration;
pub(super) use registration::committed_runtime;

// These tests begin with a committed native SessionStart and exercise subsequent ingress.
pub(super) struct IngressEndpoint {
    _home: tempfile::TempDir,
    socket_root: IsolatedTestSocketRoot,
    pub(super) paths: Paths,
    pub(super) namespace: WorkerNamespace,
    assets: IngressTestAssets,
    shell: crate::process::ProcessIdentity,
    route_revision: AtomicU64,
}

impl IngressEndpoint {
    pub(super) fn new(home_directory: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().canonicalize().unwrap();
        let socket_root = IsolatedTestSocketRoot::create().unwrap();
        let paths = Paths::new_with_socket_root(
            root.join(home_directory),
            socket_root.path().to_path_buf(),
        );
        paths.ensure_session(Fixture::terminal_id()).unwrap();
        let namespace = WorkerNamespace::capture(&paths, Fixture::terminal_id()).unwrap();
        let assets = IngressTestAssets::new(&root);
        let parent = u32::try_from(unsafe { libc::getppid() }).unwrap();
        let shell = crate::process::ProcessIdentity::capture(parent).unwrap();
        Self {
            _home: home,
            socket_root,
            paths,
            namespace,
            assets,
            shell,
            route_revision: AtomicU64::default(),
        }
    }

    pub(super) fn socket_path(&self) -> PathBuf {
        self.namespace
            .codex_hook_socket(Fixture::terminal_id())
            .unwrap()
    }

    pub(super) fn gate_next_process_scan(
        &self,
    ) -> Arc<super::ingress_live_source::BlockingProcessSourceGate> {
        self.assets.live_source().gate_next_fenced_descendants()
    }

    pub(super) fn admission_path(&self, fixture: &Fixture) -> PathBuf {
        let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
            &self.paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            fixture.identity.invocation_id,
        )
        .unwrap();
        crate::codex_hooks::CodexAdmissionStorage::for_identity(&artifacts, &fixture.identity)
            .unwrap()
            .path()
            .to_path_buf()
    }

    pub(super) fn admission_context(
        &self,
        fixture: &Fixture,
    ) -> crate::codex_hooks::CodexAdmissionContext {
        self.assets.request(fixture)
    }

    pub(super) async fn start(
        &self,
        supervisor: &CodexHookIngressSupervisor,
        fixture: &Fixture,
    ) -> Arc<super::super::runtime::CodexHookRuntime> {
        self.try_start(supervisor, fixture).await.unwrap()
    }

    pub(super) async fn try_start(
        &self,
        supervisor: &CodexHookIngressSupervisor,
        fixture: &Fixture,
    ) -> anyhow::Result<Arc<super::super::runtime::CodexHookRuntime>> {
        let router = self.start_router(supervisor, fixture).await?;
        Ok(committed_runtime(router, fixture))
    }

    pub(super) async fn bind_server(
        &self,
        supervisor: &CodexHookIngressSupervisor,
        fixture: &Fixture,
    ) -> super::super::server::CodexHookServer {
        self.try_bind_server(supervisor, fixture).await.unwrap()
    }

    pub(super) async fn try_bind_server(
        &self,
        supervisor: &CodexHookIngressSupervisor,
        fixture: &Fixture,
    ) -> anyhow::Result<super::super::server::CodexHookServer> {
        self.seed_registration(fixture)?;
        let socket = self.socket_path();
        supervisor
            .bind_test(
                &self.namespace,
                &self.paths,
                Fixture::terminal_id(),
                Fixture::runtime_epoch(),
                self.shell,
                self.assets.hook_executable(),
                self.assets.process_source(fixture, self.shell, &socket),
            )
            .await
    }

    pub(super) async fn bound_runtime(
        &self,
        server: &super::super::server::CodexHookServer,
        fixture: &Fixture,
    ) -> Arc<super::super::runtime::CodexHookRuntime> {
        committed_runtime(server.router(), fixture)
    }

    pub(super) async fn probe_listener(&self, socket: &Path) {
        drop(
            crate::local_unix::LocalUnixStream::connect(socket)
                .await
                .unwrap(),
        );
    }

    pub(super) fn restart_runtime(
        &self,
        fixture: &Fixture,
    ) -> (
        Arc<super::super::runtime::CodexHookRuntime>,
        super::super::runtime::ReplayReport,
    ) {
        super::super::runtime::CodexHookRuntime::new_test(
            &self.paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            self.shell,
            fixture.identity.clone(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            self.assets
                .process_source(fixture, self.shell, &self.socket_path()),
        )
        .unwrap()
    }

    pub(super) fn finish(mut self) {
        drop(self.namespace);
        self.socket_root.finalize().unwrap();
    }
}

#[path = "ingress_test_exchange.rs"]
mod exchange;
pub(super) use exchange::{CleanupEvidence, PendingIngressExchange};
