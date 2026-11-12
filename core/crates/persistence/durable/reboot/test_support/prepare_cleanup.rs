use super::constants::PARTICIPANT_CONFLICT;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;

const CLEANUP_CONTEXT: &str = "rollback durable reboot preparation";
const LATER_FAILURE_CONTEXT: &str = "additional durable reboot cleanup failure";

struct Artifact {
    entry: PrivateFileEntry,
    identity: FileIdentity,
}

pub(super) struct PrepareCleanup<'a> {
    paths: &'a Paths,
    guard: &'a SessionDurableMutationGuard<'a>,
    authority: &'a LoadedPreparationAuthority,
    artifacts: Vec<Artifact>,
    armed: bool,
}

impl<'a> PrepareCleanup<'a> {
    pub(super) fn new(
        paths: &'a Paths,
        guard: &'a SessionDurableMutationGuard<'a>,
        authority: &'a LoadedPreparationAuthority,
    ) -> Self {
        Self {
            paths,
            guard,
            authority,
            artifacts: Vec::new(),
            armed: true,
        }
    }

    pub(super) fn track(&mut self, entry: PrivateFileEntry, identity: FileIdentity) {
        self.artifacts.push(Artifact { entry, identity });
    }

    pub(super) fn disarm(mut self) {
        self.armed = false;
    }

    pub(super) fn fail<T>(mut self, original: anyhow::Error) -> Result<T> {
        let cleanup = self.rollback();
        self.armed = false;
        match cleanup {
            Ok(()) => Err(original),
            Err(error) => Err(original.context(format!("{CLEANUP_CONTEXT}: {error:#}"))),
        }
    }

    fn rollback(&mut self) -> Result<()> {
        let mut failure = None;
        let artifacts = std::mem::take(&mut self.artifacts);
        for artifact in artifacts.into_iter().rev() {
            let result = self.remove_artifact(artifact);
            if let Err(error) = result {
                remember_failure(&mut failure, error);
                break;
            }
        }
        if failure.is_none() {
            if let Err(error) =
                super::preparation_authority::retire(self.paths, self.guard, self.authority)
            {
                remember_failure(&mut failure, error);
            }
        }
        failure.map_or(Ok(()), |error| Err(error.context(PARTICIPANT_CONFLICT)))
    }

    fn remove_artifact(&self, artifact: Artifact) -> Result<()> {
        super::preparation_authority::fence_no_intent(self.paths, self.guard, self.authority)?;
        anyhow::ensure!(
            artifact
                .entry
                .remove_if_exact(artifact.identity, MAX_DURABLE_JSON_BYTES)?,
            PARTICIPANT_CONFLICT
        );
        super::preparation_authority::fence_no_intent(self.paths, self.guard, self.authority)
    }
}

impl Drop for PrepareCleanup<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.rollback();
        }
    }
}

pub(super) fn cleanup_unpublished<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &super::schema::RebootMigrationIntent,
    original: anyhow::Error,
) -> Result<T> {
    let cleanup = super::preparation_recovery::recover_expected_orphan(
        paths,
        guard,
        intent.preparation_authority_version,
    );
    match cleanup {
        Ok(_) => Err(original),
        Err(error) => Err(original.context(format!("{CLEANUP_CONTEXT}: {error:#}"))),
    }
}

fn remember_failure(failure: &mut Option<anyhow::Error>, error: anyhow::Error) {
    *failure = Some(match failure.take() {
        Some(previous) => previous.context(format!("{LATER_FAILURE_CONTEXT}: {error:#}")),
        None => error,
    });
}
