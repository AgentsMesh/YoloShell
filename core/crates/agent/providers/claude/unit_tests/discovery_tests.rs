use super::{constants, discovery};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

const COMPONENT_PREFIX: &str = "scope";
const NON_TRANSCRIPT_EXTENSION: &str = "txt";
const LIMIT_SENTINEL_COUNT: usize = 1;
const FIRST_UUID_VALUE: u128 = 1;

#[test]
fn ancestor_scope_overflow_fails_instead_of_truncating() {
    let directory = tempdir().unwrap();
    let cwd = (0..constants::MAX_PROJECT_ANCESTOR_SCOPES)
        .fold(PathBuf::from(Path::new("/")), |path, index| {
            path.join(format!("{COMPONENT_PREFIX}-{index}"))
        });
    let error = discovery::evidence_files(directory.path(), &cwd).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(constants::PROJECT_ANCESTOR_LIMIT_ERROR)
    );
}

#[test]
fn directory_entry_overflow_fails_deterministically() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join(COMPONENT_PREFIX);
    let project = discovery::project_directory(directory.path(), &cwd);
    fs::create_dir_all(&project).unwrap();
    let count = constants::MAX_PROJECT_DIRECTORY_ENTRIES + LIMIT_SENTINEL_COUNT;
    for index in 0..count {
        fs::write(
            project.join(format!(
                "{COMPONENT_PREFIX}-{index}.{NON_TRANSCRIPT_EXTENSION}"
            )),
            [],
        )
        .unwrap();
    }
    let error = discovery::evidence_files(directory.path(), &cwd).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(constants::DISCOVERY_ENTRY_LIMIT_ERROR)
    );
}

#[test]
fn transcript_overflow_fails_instead_of_returning_a_partial_set() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join(COMPONENT_PREFIX);
    let project = discovery::project_directory(directory.path(), &cwd);
    fs::create_dir_all(&project).unwrap();
    let count = constants::MAX_PROJECT_TRANSCRIPTS + LIMIT_SENTINEL_COUNT;
    let last = u128::try_from(count).unwrap();
    for value in FIRST_UUID_VALUE..=last {
        let session_id = Uuid::from_u128(value).hyphenated().to_string();
        fs::write(
            project.join(format!(
                "{session_id}.{}",
                crate::agent::constants::JSONL_EXTENSION
            )),
            [],
        )
        .unwrap();
    }
    let error = discovery::evidence_files(directory.path(), &cwd).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(constants::DISCOVERY_FILE_LIMIT_ERROR)
    );
}
