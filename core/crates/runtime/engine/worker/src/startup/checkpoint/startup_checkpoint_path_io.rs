use super::{StartupCheckpointEvidence, io};
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;
use uuid::Uuid;

pub(crate) fn read_path(
    paths: &Paths,
    terminal_id: Uuid,
) -> Result<Option<(StartupCheckpointEvidence, FileIdentity)>> {
    let path = paths.worker_startup_checkpoint(terminal_id);
    let entry = PrivateFileEntry::open(&path)?;
    io::read_optional(entry, || crate::paths::validate_home(paths.home()))
}

pub(crate) fn replace_path_with<F>(
    paths: &Paths,
    expected: FileIdentity,
    mut replacement: StartupCheckpointEvidence,
    before_cas: F,
) -> Result<()>
where
    F: FnOnce(
        &super::mutation::CheckpointMutation<'_>,
        &mut StartupCheckpointEvidence,
    ) -> Result<()>,
{
    let session = paths.session_dir(replacement.terminal_id);
    let namespace = crate::namespace_lease::NamespaceLease::capture(&[(
        super::constants::CHECKPOINT_NAMESPACE_LABEL,
        &session,
    )])?;
    let mutation =
        super::mutation::CheckpointMutation::acquire(paths, &namespace, replacement.terminal_id)?;
    let (_, current_identity) = mutation
        .read_optional()?
        .ok_or_else(|| anyhow::anyhow!(super::constants::EVIDENCE_MISSING_ERROR))?;
    anyhow::ensure!(current_identity == expected, super::constants::STALE_ERROR);
    before_cas(&mutation, &mut replacement)?;
    super::validation::validate(&replacement)?;
    mutation.require_no_pending_intent()?;
    mutation.write(Some(expected), &replacement).map(|_| ())
}

pub(crate) fn replace_path(
    paths: &Paths,
    expected: FileIdentity,
    replacement: &StartupCheckpointEvidence,
) -> Result<()> {
    replace_path_with(paths, expected, replacement.clone(), |_, _| Ok(()))
}
