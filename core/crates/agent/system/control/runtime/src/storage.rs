use crate::{JournalError, constants::*};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_paths::{Paths, directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub(crate) struct Locations {
    pub anchor: PathBuf,
    pub journal: PathBuf,
}
impl Locations {
    pub fn new(session: &Path, epoch: Uuid) -> Self {
        let journals = session.join(JOURNAL_DIRECTORY);
        Self {
            anchor: session
                .join(format!("{ANCHOR_PREFIX}{epoch}"))
                .with_extension(ANCHOR_EXTENSION),
            journal: journals.join(epoch.to_string()),
        }
    }
}

pub(crate) fn retain(
    session: PrivateDirectory,
    paths: &Paths,
    id: Uuid,
) -> Result<NamespaceLease, JournalError> {
    let expected = yolo_paths::platform::platform_path(&paths.session_dir(id));
    if yolo_paths::platform::platform_path(session.path()) != expected {
        return Err(JournalError::ScopeMismatch);
    }
    NamespaceLease::from_directories(vec![(SESSION_NAMESPACE, session)])
        .map_err(|_| JournalError::Unavailable)
}

pub(crate) fn poisons(error: JournalError) -> bool {
    matches!(
        error,
        JournalError::Unavailable | JournalError::OutcomeUnknown | JournalError::Poisoned
    )
}
