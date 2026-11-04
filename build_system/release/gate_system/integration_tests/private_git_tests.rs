use super::support::error_text;
use super::test_constants::{GIT_LOOSE_OBJECT_PREFIX_LENGTH, GIT_QUIET_ARGUMENT};
use crate::private_git::PrivateGit;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use yolo_release_contracts::{
    GIT_DIRECTORY_NAME, PRIVATE_DIRECTORY_MODE, PRIVATE_FILE_MODE, SOURCE_ARCHIVE_NAME,
    TEMP_DIRECTORY_NAME,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{GIT_EXECUTABLE, GIT_INIT_VERB};

const TRACKED_FILE: &str = "tracked.txt";
const TRACKED_BYTES: &[u8] = b"private pack fixture\n";
const CORRUPT_BYTE: u8 = 0;

#[test]
fn private_pack_survives_live_object_storage_move() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let fixture = PrivateFixture::materialize(&state);
    let objects = repository.path().join(".git/objects");
    let moved = repository.path().join(".git/moved-objects");
    fs::rename(&objects, &moved).unwrap();
    fs::create_dir(&objects).unwrap();
    fixture.private_git.ensure_complete().unwrap();
    fixture.private_git.export().unwrap();
    assert!(fixture.root.path().join(SOURCE_ARCHIVE_NAME).is_file());
}

#[test]
fn private_pack_ignores_live_alternates_mutation() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let fixture = PrivateFixture::materialize(&state);
    let info = repository.path().join(".git/objects/info");
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join("alternates"), b"/untrusted/live/objects\n").unwrap();
    fixture.private_git.ensure_complete().unwrap();
    fixture.private_git.export().unwrap();
}

#[test]
fn missing_live_object_is_rejected_during_materialization() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let object = git(repository.path(), &["rev-parse", "HEAD:tracked.txt"]);
    let object = object.trim();
    let path = repository
        .path()
        .join(".git/objects")
        .join(&object[..GIT_LOOSE_OBJECT_PREFIX_LENGTH])
        .join(&object[GIT_LOOSE_OBJECT_PREFIX_LENGTH..]);
    fs::rename(&path, path.with_extension("missing")).unwrap();
    let fixture = EmptyFixture::new();
    assert!(
        error_text(PrivateGit::materialize(
            &state,
            &fixture.root_handle,
            &fixture.git,
            &fixture.temporary,
        ))
        .contains("command failed")
    );
}

#[test]
fn corrupt_private_pack_is_rejected() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let fixture = PrivateFixture::materialize(&state);
    let pack = fixture.private_git.pack_path();
    fs::set_permissions(&pack, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&pack)
        .unwrap();
    let mut first = [0_u8; 1];
    file.read_exact(&mut first).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[first[0] ^ CORRUPT_BYTE.wrapping_add(1)])
        .unwrap();
    file.sync_all().unwrap();
    assert!(error_text(fixture.private_git.ensure_complete()).contains("command failed"));
}

struct PrivateFixture {
    root: tempfile::TempDir,
    private_git: PrivateGit,
}

impl PrivateFixture {
    fn materialize(state: &WorkspaceState) -> Self {
        let empty = EmptyFixture::new();
        let private_git =
            PrivateGit::materialize(state, &empty.root_handle, &empty.git, &empty.temporary)
                .unwrap();
        Self {
            root: empty.root,
            private_git,
        }
    }
}

struct EmptyFixture {
    root: tempfile::TempDir,
    root_handle: DirectoryHandle,
    git: DirectoryHandle,
    temporary: DirectoryHandle,
}

impl EmptyFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(
            root.path(),
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        for name in [GIT_DIRECTORY_NAME, TEMP_DIRECTORY_NAME] {
            fs::create_dir(root.path().join(name)).unwrap();
        }
        let root_handle = DirectoryHandle::open(root.path()).unwrap();
        let git = DirectoryHandle::open_at(&root_handle, GIT_DIRECTORY_NAME).unwrap();
        let temporary = DirectoryHandle::open_at(&root_handle, TEMP_DIRECTORY_NAME).unwrap();
        Self {
            root,
            root_handle,
            git,
            temporary,
        }
    }
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &[GIT_INIT_VERB, GIT_QUIET_ARGUMENT]);
    git(root.path(), &["config", "user.name", "Yolo Test"]);
    git(
        root.path(),
        &["config", "user.email", "yolo@example.invalid"],
    );
    fs::write(root.path().join(TRACKED_FILE), TRACKED_BYTES).unwrap();
    git(root.path(), &["add", TRACKED_FILE]);
    git(
        root.path(),
        &["commit", GIT_QUIET_ARGUMENT, "-m", "fixture"],
    );
    root
}

fn git(workspace: &Path, arguments: &[&str]) -> String {
    let output = Command::new(GIT_EXECUTABLE)
        .args(arguments)
        .current_dir(workspace)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
