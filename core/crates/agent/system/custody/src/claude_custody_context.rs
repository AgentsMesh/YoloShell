use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const HOME_LABEL: &str = "Claude custody home";
const LEDGER_PATH_INVALID: &str = "Claude custody ledger path is not canonical";
const MANIFEST_CONTEXT: &str = "read Claude custody session manifest";
const MANIFEST_GUARD_MISMATCH: &str = "Claude custody mutation guard session mismatch";
pub(super) const MANIFEST_INVALID: &str = "Claude custody session manifest is invalid";
const SESSION_LABEL: &str = "Claude custody session";
const SETTINGS_PATH_INVALID: &str = "Claude custody settings path is not canonical";

#[derive(Debug)]
pub struct ManifestAuthorityUnavailable {
    _private: (),
}

impl ManifestAuthorityUnavailable {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl std::fmt::Display for ManifestAuthorityUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(MANIFEST_INVALID)
    }
}

impl std::error::Error for ManifestAuthorityUnavailable {}

pub(super) struct ManagedCustodySession {
    paths: Paths,
    namespace: NamespaceLease,
    terminal_id: Uuid,
    ledger_path: PathBuf,
}

impl ManagedCustodySession {
    pub(super) fn capture(ledger_path: &Path) -> Result<Self> {
        let (paths, terminal_id) = resolve_ledger(ledger_path)?;
        let session = paths.session_dir(terminal_id);
        let namespace =
            NamespaceLease::capture(&[(HOME_LABEL, paths.home()), (SESSION_LABEL, &session)])
                .or_else(classify_namespace_capture_failure)?;
        Ok(Self {
            paths,
            namespace,
            terminal_id,
            ledger_path: ledger_path.to_owned(),
        })
    }

    pub(super) fn guard(&self) -> Result<SessionDurableMutationGuard<'_>> {
        SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            &self.namespace,
            self.terminal_id,
        )
    }

    pub(super) fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    pub(super) fn require_current_manifest(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
    ) -> Result<()> {
        ensure!(
            guard.terminal_id() == self.terminal_id,
            MANIFEST_GUARD_MISMATCH
        );
        let path = self.paths.manifest(self.terminal_id);
        if guard.namespace().file_is_missing(&path)? {
            return Err(anyhow::Error::new(ManifestAuthorityUnavailable::new()));
        }
        let manifest: crate::model::TerminalManifest = match guard.namespace().read_json(&path) {
            Ok(manifest) => manifest,
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) =>
            {
                return Err(anyhow::Error::new(ManifestAuthorityUnavailable::new()));
            }
            Err(error) => return Err(error).context(MANIFEST_CONTEXT),
        };
        if manifest.schema_version != crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            || manifest.terminal_id != self.terminal_id
            || manifest.archived
        {
            return Err(anyhow::Error::new(ManifestAuthorityUnavailable::new()));
        }
        self.namespace.verify()?;
        Ok(())
    }
}

fn classify_namespace_capture_failure(error: anyhow::Error) -> Result<NamespaceLease> {
    let identity_lost = error.chain().any(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| {
                crate::paths::directory::raw_errno_proves_path_identity_loss(error.raw_os_error())
            })
    });
    if identity_lost {
        return Err(anyhow::Error::new(ManifestAuthorityUnavailable::new()));
    }
    Err(error)
}

pub(super) fn ledger_path_for_settings(settings: &Path) -> Result<PathBuf> {
    let session = settings.parent().context(SETTINGS_PATH_INVALID)?;
    let candidate = session.join(super::LEDGER_FILE);
    let (paths, terminal_id) = resolve_ledger(&candidate)?;
    ensure!(
        settings
            == paths
                .session_dir(terminal_id)
                .join(crate::claude_settings_artifact::DIRECTORY_NAME),
        SETTINGS_PATH_INVALID
    );
    Ok(candidate)
}

pub(super) fn ledger_has_managed_shape(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == super::LEDGER_FILE)
        && session_parts(path).is_some_and(|(_, sessions, _)| {
            sessions
                .parent()
                .is_some_and(|home| Paths::new(home.to_owned()).sessions_dir() == sessions)
        })
}

#[cfg(test)]
pub(super) fn settings_has_managed_shape(path: &Path) -> bool {
    ledger_has_managed_shape(&path.with_file_name(super::LEDGER_FILE))
}

pub(super) fn resolve_ledger(path: &Path) -> Result<(Paths, Uuid)> {
    crate::paths::validation::validate_absolute(path)?;
    ensure!(ledger_has_managed_shape(path), LEDGER_PATH_INVALID);
    let (session, sessions, name) = session_parts(path).context(LEDGER_PATH_INVALID)?;
    let terminal_id = Uuid::parse_str(name.to_str().context(LEDGER_PATH_INVALID)?)?;
    let home = sessions.parent().context(LEDGER_PATH_INVALID)?;
    crate::paths::validate_home(home)?;
    let paths = Paths::new(home.to_owned());
    ensure!(
        session == paths.session_dir(terminal_id)
            && path == paths.session_dir(terminal_id).join(super::LEDGER_FILE)
            && name == terminal_id.hyphenated().to_string().as_str(),
        LEDGER_PATH_INVALID
    );
    Ok((paths, terminal_id))
}

fn session_parts(path: &Path) -> Option<(&Path, &Path, &std::ffi::OsStr)> {
    let session = path.parent()?;
    Some((session, session.parent()?, session.file_name()?))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_context_tests.rs"]
mod tests;
