use super::{ClaudeGenerationStore, constants::*};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateDirectoryLock;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

#[path = "mutation_identity.rs"]
mod identity;
#[path = "mutation_hook.rs"]
mod test_hook;

#[cfg(test)]
pub(super) use test_hook::set_test_hook;

pub(super) struct GenerationMutation<'a> {
    guard: Option<SessionDurableMutationGuard<'a>>,
    manifest_paths: Option<Paths>,
    _fixture_lock: Option<PrivateDirectoryLock>,
}

impl<'a> GenerationMutation<'a> {
    pub(super) fn acquire(
        store: &ClaudeGenerationStore,
        namespace: &'a NamespaceLease,
    ) -> Result<Self> {
        let Some((paths, session_id)) = identity::managed_session(store)? else {
            #[cfg(test)]
            return Self::unmanaged_test(store, namespace);
            #[cfg(not(test))]
            anyhow::bail!(GENERATION_PATH_INVALID);
        };
        let guard =
            SessionDurableMutationGuard::acquire_participant(&paths, namespace, session_id)?;
        identity::require_current_manifest(&guard, &paths, session_id)?;
        guard.require_no_pending_intent()?;
        Ok(Self {
            guard: Some(guard),
            manifest_paths: Some(paths),
            _fixture_lock: None,
        })
    }

    pub(super) fn write<T, F>(
        &self,
        store: &ClaudeGenerationStore,
        namespace: &NamespaceLease,
        expected: Option<FileIdentity>,
        next: &T,
        fence: F,
    ) -> Result<()>
    where
        T: serde::Serialize,
        F: FnMut() -> Result<()>,
    {
        self.write_path(namespace, &store.path, expected, next, fence)
    }

    pub(super) fn write_path<T, F>(
        &self,
        namespace: &NamespaceLease,
        path: &Path,
        expected: Option<FileIdentity>,
        next: &T,
        mut fence: F,
    ) -> Result<()>
    where
        T: serde::Serialize,
        F: FnMut() -> Result<()>,
    {
        if let Some(guard) = self.guard.as_ref() {
            return guard.write_json_expected_with_fence(path, expected, next, |phase| {
                self.require_current()?;
                test_hook::run(phase)?;
                self.require_current()?;
                fence()?;
                self.require_current()?;
                if phase == yolo_paths::durable_mutation::DurableMutationFencePhase::Commit {
                    require_expected_target(namespace, path, expected)?;
                }
                Ok(())
            });
        }
        namespace.write_json_expected_with_commit_fence(path, expected, next, || {
            fence()?;
            require_expected_target(namespace, path, expected)
        })?;
        Ok(())
    }

    pub(super) fn require_current(&self) -> Result<()> {
        let guard = self.guard.as_ref().context(GENERATION_PATH_INVALID)?;
        let paths = self
            .manifest_paths
            .as_ref()
            .context(GENERATION_PATH_INVALID)?;
        identity::require_current_manifest(guard, paths, guard.terminal_id())?;
        guard.require_no_pending_intent()
    }

    #[cfg(test)]
    fn unmanaged_test(
        store: &ClaudeGenerationStore,
        namespace: &'a NamespaceLease,
    ) -> Result<Self> {
        namespace.verify()?;
        Ok(Self {
            guard: None,
            manifest_paths: None,
            _fixture_lock: Some(store.lock_binding_lifecycle()?),
        })
    }
}

fn require_expected_target(
    namespace: &NamespaceLease,
    path: &Path,
    expected: Option<FileIdentity>,
) -> Result<()> {
    match expected {
        None => {
            ensure!(namespace.file_is_missing(path)?, GENERATION_TARGET_CHANGED);
            Ok(())
        }
        Some(expected) => {
            let (_, current) = namespace.read_json_with_identity::<serde_json::Value>(path)?;
            ensure!(current == expected, GENERATION_TARGET_CHANGED);
            Ok(())
        }
    }
}
