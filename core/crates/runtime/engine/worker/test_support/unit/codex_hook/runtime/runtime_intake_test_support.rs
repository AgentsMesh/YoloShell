use super::support::Fixture;
use crate::codex_hooks::{
    CodexAdmissionContext, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
};
use crate::worker::codex_hook::router::{
    CodexRouteLease, CodexSessionRouter, DurableCodexAdmission,
};
use std::ops::Deref;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const EXECUTABLE_DIRECTORY: &str = "routed-intake-executables";
const CWD_DIRECTORY: &str = "routed-intake-project";
const DATA_HOME_DIRECTORY: &str = "routed-intake-data-home";
const HOOK_FILE_NAME: &str = "routed-intake-hook";
const REAL_FILE_NAME: &str = "routed-intake-real";
const ADMISSION_REPLACEMENT_FILE_NAME: &str = "admission-replacement.json";
const EXECUTABLE_CONTENT: &[u8] = b"#!/bin/sh\nexit 0\n";
const PRIVATE_FILE_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;

pub(super) struct RoutedFixture {
    base: Fixture,
    router: CodexSessionRouter,
    pub(super) record: CodexAdmissionRecord,
    pub(super) storage: CodexAdmissionStorage,
}

impl RoutedFixture {
    pub(super) fn new() -> Self {
        let base = Fixture::new_isolated();
        let root = base.paths.home().parent().unwrap().canonicalize().unwrap();
        let executable_directory = directory(&root.join(EXECUTABLE_DIRECTORY));
        let request = CodexAdmissionContext::new(
            base.identity.clone(),
            directory(&root.join(CWD_DIRECTORY)),
            directory(&root.join(DATA_HOME_DIRECTORY)),
            executable(&executable_directory.join(HOOK_FILE_NAME)),
            executable(&executable_directory.join(REAL_FILE_NAME)),
        );
        let record = CodexAdmissionRecord::capture_for_test(&request).unwrap();
        let artifacts = artifacts(&base);
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &base.identity).unwrap();
        let router = install_router(&base, &storage, &record, &base.runtime);
        Self {
            base,
            router,
            record,
            storage,
        }
    }

    pub(super) fn reconcile(&self) -> anyhow::Result<super::super::runtime::IntakeReplayReport> {
        self.runtime.reconcile_routed_intake(&self.router)
    }

    pub(super) fn router_for(
        &self,
        runtime: &Arc<super::super::runtime::CodexHookRuntime>,
    ) -> CodexSessionRouter {
        install_router(&self.base, &self.storage, &self.record, runtime)
    }

    pub(super) fn router(&self) -> &CodexSessionRouter {
        &self.router
    }

    pub(super) fn provisional_router(&self) -> (CodexSessionRouter, CodexRouteLease) {
        let publication = self.storage.publish_for_test(&self.record).unwrap();
        let admission =
            DurableCodexAdmission::from_publication(self.storage.clone(), publication).unwrap();
        let router = CodexSessionRouter::new(
            self.paths.clone(),
            self.identity.terminal_id,
            self.identity.runtime_epoch,
        )
        .unwrap();
        let lease = router
            .install_or_replay(admission, |_| Ok(Arc::clone(&self.runtime)))
            .unwrap();
        assert!(!lease.is_replay());
        (router, lease)
    }

    pub(super) fn replace_admission_same_payload(&self) {
        let bytes = serde_json::to_vec_pretty(&self.record).unwrap();
        let replacement = self
            .storage
            .path()
            .parent()
            .unwrap()
            .join(ADMISSION_REPLACEMENT_FILE_NAME);
        std::fs::write(&replacement, bytes).unwrap();
        std::fs::set_permissions(
            &replacement,
            std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
        )
        .unwrap();
        std::fs::rename(replacement, self.storage.path()).unwrap();
    }

    pub(super) fn replay_lease(&self) -> CodexRouteLease {
        let publication = self.storage.publish_for_test(&self.record).unwrap();
        let admission =
            DurableCodexAdmission::from_publication(self.storage.clone(), publication).unwrap();
        self.router
            .install_or_replay(admission, |_| unreachable!())
            .unwrap()
    }
}

impl Deref for RoutedFixture {
    type Target = Fixture;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn install_router(
    fixture: &Fixture,
    storage: &CodexAdmissionStorage,
    record: &CodexAdmissionRecord,
    runtime: &Arc<super::super::runtime::CodexHookRuntime>,
) -> CodexSessionRouter {
    let publication = storage.publish_for_test(record).unwrap();
    let admission = DurableCodexAdmission::from_publication(storage.clone(), publication).unwrap();
    let router = CodexSessionRouter::new(
        fixture.paths.clone(),
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
    .unwrap();
    router
        .install_or_replay(admission, |_| Ok(Arc::clone(runtime)))
        .unwrap()
        .settle()
        .unwrap();
    router
}

fn artifacts(fixture: &Fixture) -> CodexHookArtifacts {
    CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap()
}

pub(super) fn directory(path: &Path) -> PathBuf {
    std::fs::create_dir(path).unwrap();
    path.to_path_buf()
}

pub(super) fn executable(path: &Path) -> PathBuf {
    std::fs::write(path, EXECUTABLE_CONTENT).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::constants::RUNTIME_EXECUTABLE_MODE),
    )
    .unwrap();
    path.to_path_buf()
}
