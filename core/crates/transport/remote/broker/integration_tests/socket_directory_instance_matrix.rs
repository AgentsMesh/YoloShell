use super::*;
use crate::paths::directory::PrivateDirectory;
use std::cell::{Cell, RefCell};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const TARGET_SOCKET: &str = "target.sock";
const QUARANTINE_SOCKET: &str = ".quarantine.sock";
const RETRY_SOCKET: &str = ".quarantine-retry.sock";
const ACTION_FAILURE: &str = "injected quarantine action failure";
const INITIAL_ATTEMPT: usize = 0;
const ATTEMPT_INCREMENT: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Success,
    NameFailure,
    CollisionThenSuccess,
    CollisionExhaustion,
    HookFailure,
    HookRestoreCollision,
    IdentityChanged,
}

#[test]
fn one_quarantine_instance_covers_success_failure_retry_and_restore() {
    for action in [
        Action::Success,
        Action::NameFailure,
        Action::CollisionThenSuccess,
        Action::CollisionExhaustion,
        Action::HookFailure,
        Action::HookRestoreCollision,
        Action::IdentityChanged,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(STATE_DIRECTORY);
        let directory = SocketDirectory::new(PrivateDirectory::ensure(&path).unwrap());
        let name = CString::new(TARGET_SOCKET).unwrap();
        let original = bind_socket(&path.join(TARGET_SOCKET));
        let expected = directory.identity(&name).unwrap().unwrap();
        let collision = collision_for(action, &path);
        let replacement = RefCell::new(None);
        let attempts = Cell::new(INITIAL_ATTEMPT);
        let mut next_name = || {
            let attempt = attempts.get();
            attempts.set(attempt + ATTEMPT_INCREMENT);
            let selected = match action {
                Action::NameFailure => anyhow::bail!(ACTION_FAILURE),
                Action::CollisionThenSuccess if attempt > INITIAL_ATTEMPT => RETRY_SOCKET,
                _ => QUARANTINE_SOCKET,
            };
            Ok(CString::new(selected).unwrap())
        };

        let result = directory.remove_expected_using(
            &name,
            expected,
            || match action {
                Action::HookFailure => anyhow::bail!(ACTION_FAILURE),
                Action::HookRestoreCollision => {
                    replacement.replace(Some(bind_socket(&path.join(TARGET_SOCKET))));
                    anyhow::bail!(ACTION_FAILURE)
                }
                Action::IdentityChanged => {
                    let quarantined = path.join(QUARANTINE_SOCKET);
                    fs::remove_file(&quarantined)?;
                    replacement.replace(Some(bind_socket(&quarantined)));
                    Ok(())
                }
                _ => Ok(()),
            },
            &mut next_name,
        );

        if matches!(action, Action::Success | Action::CollisionThenSuccess) {
            result.unwrap();
            assert!(!path.join(TARGET_SOCKET).exists());
        } else {
            assert!(result.is_err(), "action {action:?} unexpectedly succeeded");
        }
        drop(replacement);
        drop(collision);
        drop(original);
    }
}

fn collision_for(action: Action, path: &Path) -> Option<std::os::unix::net::UnixListener> {
    matches!(
        action,
        Action::CollisionThenSuccess | Action::CollisionExhaustion
    )
    .then(|| bind_socket(&path.join(QUARANTINE_SOCKET)))
}

fn bind_socket(path: &PathBuf) -> std::os::unix::net::UnixListener {
    let listener = std::os::unix::net::UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}
