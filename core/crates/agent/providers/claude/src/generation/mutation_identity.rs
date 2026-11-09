use super::super::{ClaudeGenerationStore, constants::*};
use crate::paths::Paths;
use anyhow::{Context, Result, ensure};
use uuid::Uuid;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

pub(super) fn managed_session(store: &ClaudeGenerationStore) -> Result<Option<(Paths, Uuid)>> {
    let Some(session) = store.path.parent() else {
        return Ok(None);
    };
    let Some(sessions) = session.parent() else {
        return Ok(None);
    };
    let Some(home) = sessions.parent() else {
        return Ok(None);
    };
    let Some(name) = session.file_name().and_then(std::ffi::OsStr::to_str) else {
        return Ok(None);
    };
    let Ok(session_id) = Uuid::parse_str(name) else {
        return Ok(None);
    };
    let paths = Paths::new(home.to_owned());
    if sessions != paths.sessions_dir()
        || session != paths.session_dir(session_id)
        || store.path != session.join(AUTHORITY_FILE_NAME)
    {
        return Ok(None);
    }
    Ok(Some((paths, session_id)))
}

pub(super) fn require_current_manifest(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    session_id: Uuid,
) -> Result<()> {
    let manifest: crate::model::TerminalManifest = guard
        .namespace()
        .read_json(&paths.manifest(session_id))
        .context(GENERATION_MANIFEST_CONTEXT)?;
    ensure!(
        manifest.schema_version
            == yolo_application_support::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == session_id
            && !manifest.archived,
        GENERATION_MANIFEST_INVALID
    );
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/generation/mutation_identity_matrix_tests.rs"]
mod matrix_tests;
