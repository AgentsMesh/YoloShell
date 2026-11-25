use super::*;
use crate::paths::directory::PrivateDirectory;
use std::cell::{Cell, RefCell};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const TARGET_SOCKET: &str = "target.sock";
const STAGING_SOCKET: &str = "staging.sock";
const RETRY_SOCKET: &str = "retry.sock";
const INVALID_SOCKET: &str = "missing/staging.sock";
const ACTION_FAILURE: &str = "injected binding action failure";
const INITIAL_ATTEMPT: usize = 0;
const ATTEMPT_INCREMENT: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Success,
    BeforeFailure,
    NameFailure,
    InvalidName,
    CollisionThenSuccess,
    CollisionExhaustion,
    RestoreFailure,
    ReplaceStaging,
}

#[tokio::test]
async fn one_binding_instance_covers_success_failure_retry_and_rollback() {
    for action in [
        Action::Success,
        Action::BeforeFailure,
        Action::NameFailure,
        Action::InvalidName,
        Action::CollisionThenSuccess,
        Action::CollisionExhaustion,
        Action::RestoreFailure,
        Action::ReplaceStaging,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(STATE_DIRECTORY);
        let directory = SocketDirectory::new(PrivateDirectory::ensure(&path).unwrap());
        let collision = collision_for(action, &path);
        let replacement = RefCell::new(None);
        let attempts = Cell::new(INITIAL_ATTEMPT);

        let result = bind_private_using(
            &directory,
            CString::new(TARGET_SOCKET).unwrap(),
            || match action {
                Action::BeforeFailure => anyhow::bail!(ACTION_FAILURE),
                _ => Ok(()),
            },
            |name, binding| match action {
                Action::RestoreFailure => binding.inject_restore_failure(ACTION_FAILURE),
                Action::ReplaceStaging => {
                    let staging = path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
                    fs::remove_file(&staging).unwrap();
                    replacement.replace(Some(bind_socket(&staging)));
                }
                _ => {}
            },
            || {
                let attempt = attempts.get();
                attempts.set(attempt + ATTEMPT_INCREMENT);
                let name = match action {
                    Action::NameFailure => anyhow::bail!(ACTION_FAILURE),
                    Action::InvalidName => INVALID_SOCKET,
                    Action::CollisionThenSuccess if attempt > INITIAL_ATTEMPT => RETRY_SOCKET,
                    _ => STAGING_SOCKET,
                };
                Ok(CString::new(name).unwrap())
            },
        );

        if matches!(action, Action::Success | Action::CollisionThenSuccess) {
            drop(result.unwrap());
        } else {
            assert!(result.is_err(), "action {action:?} unexpectedly succeeded");
        }
        assert!(!path.join(TARGET_SOCKET).exists());
        drop(replacement);
        drop(collision);
    }
}

fn collision_for(action: Action, path: &Path) -> Option<std::os::unix::net::UnixListener> {
    matches!(
        action,
        Action::CollisionThenSuccess | Action::CollisionExhaustion
    )
    .then(|| bind_socket(&path.join(STAGING_SOCKET)))
}

fn bind_socket(path: &PathBuf) -> std::os::unix::net::UnixListener {
    std::os::unix::net::UnixListener::bind(path).unwrap()
}
