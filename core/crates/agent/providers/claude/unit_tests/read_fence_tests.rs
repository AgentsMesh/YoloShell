use super::*;
use serde_json::json;
use std::fs::{self, FileTimes, OpenOptions};
use std::path::PathBuf;
use tempfile::tempdir;

const SESSION_ID: &str = "e3ef2aad-a247-4f12-8222-255e23c46b88";
const DATA_HOME_DIRECTORY: &str = "data-home";
const PROJECT_DIRECTORY: &str = "project";
const PROJECT_SCOPE: &str = "project-scope";
const DISPLACED_SCOPE: &str = "displaced-scope";
const DISPLACED_HOME: &str = "displaced-home";
const REPLACEMENT_FILE: &str = "replacement.jsonl";
const HARD_LINK_FILE: &str = "hard-link.jsonl";
const EXTRA_RECORD_TYPE: &str = "fence-regression-padding";

struct Fixture {
    _directory: tempfile::TempDir,
    cwd: PathBuf,
    scope: PathBuf,
    evidence: PathBuf,
    contents: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let cwd = directory.path().join(PROJECT_DIRECTORY);
        let scope = directory
            .path()
            .join(DATA_HOME_DIRECTORY)
            .join(constants::EVIDENCE_DIR)
            .join(PROJECT_SCOPE);
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&scope).unwrap();
        let evidence = scope.join(format!(
            "{SESSION_ID}.{}",
            crate::agent::constants::JSONL_EXTENSION
        ));
        let anchor = json!({
            (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::CWD_KEY): cwd,
        });
        let contents = format!("{anchor}\n");
        fs::write(&evidence, &contents).unwrap();
        Self {
            _directory: directory,
            cwd,
            scope,
            evidence,
            contents,
        }
    }

    fn inspect(&self, hook: impl FnOnce()) -> Option<AgentEvidence> {
        ClaudeAdapter
            .inspect_evidence_with_hook(&self.evidence, &self.cwd, hook)
            .unwrap()
    }
}

#[test]
fn in_place_mutation_after_read_is_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.evidence.clone();
    let contents = fixture.contents.clone();
    let result = fixture.inspect(|| {
        let padding = json!({
            (constants::RECORD_TYPE_KEY): EXTRA_RECORD_TYPE,
            (constants::SESSION_ID_KEY): SESSION_ID,
        });
        fs::write(evidence, format!("{contents}{padding}\n")).unwrap();
    });
    assert!(result.is_none());
}

#[test]
fn disappearance_and_directory_entry_replacement_are_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.evidence.clone();
    assert!(
        fixture
            .inspect(|| fs::remove_file(evidence).unwrap())
            .is_none()
    );

    let fixture = Fixture::new();
    let scope = fixture.scope.clone();
    let displaced = scope.parent().unwrap().join(DISPLACED_SCOPE);
    let contents = fixture.contents.clone();
    let evidence_name = fixture.evidence.file_name().unwrap().to_owned();
    assert!(
        fixture
            .inspect(|| {
                fs::rename(&scope, displaced).unwrap();
                fs::create_dir(&scope).unwrap();
                fs::write(scope.join(evidence_name), contents).unwrap();
            })
            .is_none()
    );
}

#[test]
fn same_length_same_mtime_inode_swap_is_rejected() {
    let fixture = Fixture::new();
    let replacement = fixture.scope.join(REPLACEMENT_FILE);
    fs::write(&replacement, &fixture.contents).unwrap();
    let original = fs::metadata(&fixture.evidence).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_times(FileTimes::new().set_modified(original.modified().unwrap()))
        .unwrap();
    let replacement_metadata = fs::metadata(&replacement).unwrap();
    assert_eq!(replacement_metadata.len(), original.len());
    assert_eq!(
        replacement_metadata.modified().unwrap(),
        original.modified().unwrap()
    );
    let evidence = fixture.evidence.clone();
    assert!(
        fixture
            .inspect(|| fs::rename(replacement, evidence).unwrap())
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn symbolic_and_hard_link_evidence_are_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let target = fixture.scope.join(REPLACEMENT_FILE);
    fs::write(&target, &fixture.contents).unwrap();
    fs::remove_file(&fixture.evidence).unwrap();
    symlink(&target, &fixture.evidence).unwrap();
    assert!(fixture.inspect(|| {}).is_none());

    let fixture = Fixture::new();
    fs::hard_link(&fixture.evidence, fixture.scope.join(HARD_LINK_FILE)).unwrap();
    assert!(fixture.inspect(|| {}).is_none());
}

#[cfg(unix)]
#[test]
fn symbolic_project_scope_is_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let displaced = fixture.scope.parent().unwrap().join(DISPLACED_SCOPE);
    fs::rename(&fixture.scope, &displaced).unwrap();
    symlink(displaced, &fixture.scope).unwrap();
    assert!(fixture.inspect(|| {}).is_none());
}

#[cfg(unix)]
#[test]
fn symbolic_or_replaced_data_home_is_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let data_home = fixture
        .scope
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let displaced = data_home.parent().unwrap().join(DISPLACED_HOME);
    fs::rename(&data_home, &displaced).unwrap();
    symlink(&displaced, &data_home).unwrap();
    assert!(fixture.inspect(|| {}).is_none());

    let fixture = Fixture::new();
    let data_home = fixture
        .scope
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let displaced = data_home.parent().unwrap().join(DISPLACED_HOME);
    assert!(
        fixture
            .inspect(|| {
                fs::rename(&data_home, &displaced).unwrap();
                symlink(&displaced, &data_home).unwrap();
            })
            .is_none()
    );
}
