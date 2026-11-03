use super::super::{
    binding::{self, ProjectionMode},
    test_support::{ProjectionFixture, registry},
};
use crate::agent::AgentBindingStore;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::CodexBindingAuthority;
use crate::codex_hooks::CodexSnapshotLineage;
use crate::namespace_lease::NamespaceLease;
use anyhow::Result;

const SESSION_DIRECTORY: &str = "control-projection";
const NAMESPACE_LABEL: &str = "Codex control projection fixture";

pub(super) struct ControlSource {
    pub projection: ProjectionFixture,
    _root: tempfile::TempDir,
    namespace: NamespaceLease,
    store: AgentBindingStore,
}

impl ControlSource {
    pub fn new() -> Self {
        let projection = ProjectionFixture::new();
        projection.start();
        projection.prompt();
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap().join(SESSION_DIRECTORY);
        crate::paths::directory::PrivateDirectory::ensure(&directory).unwrap();
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &directory)]).unwrap();
        let store =
            AgentBindingStore::new(directory.join(crate::agent::constants::BINDING_FILE_NAME));
        let fixture = Self {
            projection,
            _root: root,
            namespace,
            store,
        };
        fixture.persist_current();
        fixture
    }

    pub fn persist_current(&self) {
        let source = self.projection.authority.as_ref().unwrap();
        let lifecycle = source.runtime.lock_lifecycle_anyhow().unwrap();
        let reducer = lifecycle.reducer().unwrap();
        let previous = self.store.read_raw_in_namespace(&self.namespace).unwrap();
        let projected = binding::project(
            &self.projection.route(),
            reducer.state(),
            reducer.revision(),
            ProjectionMode::Active,
            previous.as_ref(),
            &registry(),
        )
        .unwrap();
        let worker = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let authority = CodexBindingAuthority::new_with_verified_resume_identity(
            AuthorityRuntimeIdentity {
                runtime_epoch: source.identity.runtime_epoch,
                worker: AuthorityProcessIdentity {
                    pid: worker.pid,
                    start_time: worker.start_time,
                },
            },
            projected.provider,
            projected.provider_version,
            projected.version,
            projected.status,
            projected.binding,
            &crate::process::boot_id(),
        )
        .unwrap();
        self.store
            .codex_authority_store()
            .unwrap()
            .publish_in_namespace_with_commit_fence(
                &self.namespace,
                self.store.path(),
                &authority,
                || Ok(()),
            )
            .unwrap();
    }

    pub fn authority(&self) -> CodexBindingAuthority {
        self.store
            .codex_authority_store()
            .unwrap()
            .load_in_namespace(&self.namespace)
            .unwrap()
            .unwrap()
    }

    pub fn run<T>(
        &self,
        callback: impl FnOnce(&CodexSnapshotLineage, &dyn Fn() -> Result<()>) -> Result<T>,
    ) -> Result<T> {
        let authority = self.authority();
        self.projection
            .router
            .with_control_route(&authority, |entry| {
                entry
                    .runtime()
                    .with_control_authority(entry, &authority, &registry(), callback)
            })
    }
}
