use super::super::support::Fixture;
use super::IngressEndpoint;
use crate::codex_hooks::{
    CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts, CodexHookIntake,
};
use anyhow::{Context, Result};
use std::sync::atomic::Ordering;

const REVISION_INCREMENT: u64 = 1;
const REVISION_EXHAUSTED: &str = "native ingress fixture route revision is exhausted";

pub(in crate::worker::codex_hook) fn committed_runtime(
    router: std::sync::Arc<crate::worker::codex_hook::router::CodexSessionRouter>,
    fixture: &Fixture,
) -> std::sync::Arc<crate::worker::codex_hook::runtime::CodexHookRuntime> {
    let lease = router.lease_committed(&fixture.identity).unwrap();
    let runtime = std::sync::Arc::clone(lease.entry().runtime());
    lease.settle().unwrap();
    runtime
}

impl IngressEndpoint {
    pub(in crate::worker::codex_hook::tests) async fn start_router(
        &self,
        supervisor: &crate::worker::codex_hook::ingress_supervisor::CodexHookIngressSupervisor,
        fixture: &Fixture,
    ) -> Result<std::sync::Arc<crate::worker::codex_hook::router::CodexSessionRouter>> {
        self.seed_registration(fixture)?;
        let socket = self.socket_path();
        supervisor
            .start_test(
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

    pub(super) fn seed_registration(&self, fixture: &Fixture) -> Result<()> {
        let context = self.assets.request(fixture);
        let owner = &context.identity;
        let artifacts = CodexHookArtifacts::new(
            &self.paths,
            owner.terminal_id,
            owner.runtime_epoch,
            owner.invocation_id,
        )?;
        let storage = CodexAdmissionStorage::for_identity(&artifacts, owner)?;
        if storage.read()?.is_some() {
            return Ok(());
        }
        let authority =
            crate::agent::AgentBindingStore::new(self.paths.agent_binding(owner.terminal_id))
                .codex_authority_store()?;
        if let Some(authority) = authority.load_in_namespace(self.namespace.lease())? {
            self.route_revision
                .fetch_max(authority.version.route_revision, Ordering::AcqRel);
        }
        let revision = self
            .route_revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |revision| {
                revision.checked_add(REVISION_INCREMENT)
            })
            .ok()
            .context(REVISION_EXHAUSTED)?
            + REVISION_INCREMENT;
        let mut first = fixture.start(crate::agent::hooks::ZERO_SEQUENCE);
        first.metadata.cwd = Some(context.cwd.clone());
        let record =
            CodexAdmissionRecord::capture_native_with_route_revision(context, revision, first)?;
        let writer = storage.acquire_retirement_writer_fence()?;
        writer.verify_unsealed()?;
        CodexHookIntake::for_identity(artifacts.intake_directory(), &record.request().identity)?;
        storage.publish_with_commit_fence(&record, || writer.verify_unsealed())?;
        Ok(())
    }
}
