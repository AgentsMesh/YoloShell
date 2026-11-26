use super::*;
use crate::paths::constants::{PRIVATE_FILE_MODE, SOCKET_ROOT_PREFIX};
use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener as StdListener;

const DIRECTORY_PREFIX: &str = "ysa-matrix-";
const MANAGED_DIRECTORY: &str = "managed";
const DISPLACED_DIRECTORY: &str = "displaced";
const SOCKET_NAME: &str = "matrix.sock";
const OTHER_DIRECTORY: &str = "other";
const ACTION_FAILURE: &str = "injected activation action failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Success,
    HookFailure,
    TargetCollision,
    StageRemoved,
    StageReplaced,
    ParentMoved,
}

#[tokio::test]
async fn one_install_instance_covers_success_failure_races_and_retained_parent() {
    for action in [
        Action::Success,
        Action::HookFailure,
        Action::TargetCollision,
        Action::StageRemoved,
        Action::StageReplaced,
        Action::ParentMoved,
    ] {
        let fixture = Fixture::new();
        let replacement = RefCell::new(None);
        let target = SocketTarget::new(fixture.parent(), &fixture.socket).unwrap();
        let stage = target.stage.stage(socket_stage::Kind::Bind).unwrap().1;
        let result = install_with_hook(target, || match action {
            Action::HookFailure => anyhow::bail!(ACTION_FAILURE),
            Action::TargetCollision => {
                replacement.replace(Some(bind_private(&fixture.socket)));
                Ok(())
            }
            Action::StageRemoved => {
                fs::remove_file(&stage)?;
                Ok(())
            }
            Action::StageReplaced => {
                fs::remove_file(&stage)?;
                replacement.replace(Some(bind_private(&stage)));
                Ok(())
            }
            Action::ParentMoved => {
                fs::rename(
                    &fixture.managed,
                    fixture.root.path().join(DISPLACED_DIRECTORY),
                )?;
                PrivateDirectory::ensure(&fixture.managed)?;
                Ok(())
            }
            Action::Success => Ok(()),
        })
        .await;

        if matches!(action, Action::Success | Action::ParentMoved) {
            let (listener, publication) = result.unwrap();
            drop(publication);
            drop(listener);
        } else {
            assert!(result.is_err(), "action {action:?} unexpectedly succeeded");
        }
    }
}

#[test]
fn target_construction_and_removal_reject_mismatched_authority() {
    let fixture = Fixture::new();
    let other = fixture.root.path().join(OTHER_DIRECTORY);
    PrivateDirectory::ensure(&other).unwrap();
    assert!(SocketTarget::new(fixture.parent(), &other.join(SOCKET_NAME)).is_err());

    let original = bind_private(&fixture.socket);
    let target = SocketTarget::new(fixture.parent(), &fixture.socket).unwrap();
    let identity = target.inspect(true).unwrap().unwrap();
    fs::remove_file(&fixture.socket).unwrap();
    let replacement = bind_private(&fixture.socket);
    assert!(target.remove(identity).is_err());
    drop((replacement, original));
}

struct Fixture {
    root: tempfile::TempDir,
    managed: PathBuf,
    socket: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let root = tempfile::Builder::new()
            .prefix(DIRECTORY_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        let managed = root.path().join(MANAGED_DIRECTORY);
        PrivateDirectory::ensure(&managed).unwrap();
        let socket = managed.join(SOCKET_NAME);
        Self {
            root,
            managed,
            socket,
        }
    }

    fn parent(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.managed).unwrap()
    }
}

fn bind_private(path: &Path) -> StdListener {
    let listener = StdListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}
