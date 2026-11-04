use super::tests_security_support::{REPLACEMENT_BYTES, set_private_directory, write_private};
use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const RETAINED_PAYLOAD: &[u8] = b"retained output";
pub(super) const SAVED_FILE_SUFFIX: &str = ".saved";
const APPEND_PHASE_FAULT: &str = "injected retained append phase failure";
const JOURNAL_FILE: &str = "journal";
const MOVED_TERMINAL_DIRECTORY: &str = "moved-terminal";
const TERMINAL_DIRECTORY: &str = "terminal";

pub(super) struct RetainedFixture {
    _temporary_root: tempfile::TempDir,
    pub(super) journal: Journal,
    pub(super) attempt: AppendAttempt,
    pub(super) moved_journal: PathBuf,
    pub(super) moved_cursor: PathBuf,
    replacement_journal: PathBuf,
    replacement_cursor: PathBuf,
}

impl RetainedFixture {
    pub(super) fn fail_at(stage: AppendStage) -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(TERMINAL_DIRECTORY);
        crate::paths::directory::PrivateDirectory::ensure(&parent).unwrap();
        let path = parent.join(JOURNAL_FILE);
        let cursor = cursor_path(&path);
        let moved_parent = root.path().join(MOVED_TERMINAL_DIRECTORY);
        let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
        let attempt = journal.append_attempt();
        let mut injected = false;
        let result = journal.append_with_test_hook(RETAINED_PAYLOAD, |current| {
            if current != stage {
                return Ok(());
            }
            fs::rename(&parent, &moved_parent)?;
            set_private_directory(&parent);
            write_private(&path, REPLACEMENT_BYTES);
            write_private(&cursor, REPLACEMENT_BYTES);
            injected = true;
            Err(std::io::Error::other(APPEND_PHASE_FAULT).into())
        });
        assert!(result.is_err());
        assert!(injected);
        Self {
            _temporary_root: root,
            journal,
            attempt,
            moved_journal: moved_parent.join(JOURNAL_FILE),
            moved_cursor: moved_parent.join(cursor.file_name().unwrap()),
            replacement_journal: path,
            replacement_cursor: cursor,
        }
    }

    pub(super) fn reconcile(&mut self) -> Result<Option<OutputChunk>, JournalError> {
        self.journal
            .reconcile_append_retained(self.attempt, RETAINED_PAYLOAD)
    }

    pub(super) fn assert_exactly_once(&mut self) {
        let runtime_epoch = self.journal.runtime_epoch();
        let mut reopened = Journal::open(&self.moved_journal, runtime_epoch).unwrap();
        let chunks = reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();
        assert_eq!(chunks.len(), INITIAL_RETAINED_SEQUENCE as usize);
        assert_eq!(chunks[0].data, RETAINED_PAYLOAD);
    }

    pub(super) fn assert_replacements_unchanged(&self) {
        assert_eq!(
            fs::read(&self.replacement_journal).unwrap(),
            REPLACEMENT_BYTES
        );
        assert_eq!(
            fs::read(&self.replacement_cursor).unwrap(),
            REPLACEMENT_BYTES
        );
    }

    pub(super) fn assert_artifacts_bounded(&self) {
        assert!(self.entry_count() <= crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES);
    }

    pub(super) fn entry_count(&self) -> usize {
        fs::read_dir(self.moved_journal.parent().unwrap())
            .unwrap()
            .flatten()
            .count()
    }

    pub(super) fn artifact(&self, prefix: &str) -> PathBuf {
        fs::read_dir(self.moved_journal.parent().unwrap())
            .unwrap()
            .flatten()
            .find(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
            .unwrap()
            .path()
    }

    pub(super) fn assert_retired_cursor(&self, expected: &[u8]) {
        let retired =
            self.artifact(crate::paths::test_support::retained_isolation_artifact_prefix());
        assert_eq!(fs::read(retired).unwrap(), expected);
    }
}

fn cursor_path(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().map(OsString::from).unwrap();
    name.push(COMMIT_FILE_SUFFIX);
    journal.with_file_name(name)
}

pub(super) fn saved_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(SAVED_FILE_SUFFIX);
    path.with_file_name(name)
}
