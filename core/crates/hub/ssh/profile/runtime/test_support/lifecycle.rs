use super::{constants::*, fixture::Fixture, lifecycle_constants::*};
use std::{
    future::Future,
    os::unix::fs::{MetadataExt, PermissionsExt},
    task::{Context, Poll, Waker},
};
use yolo_hub_ssh_profiles::SSH_PROFILES_DIRECTORY;

#[path = "lifecycle_tunnels.rs"]
mod tunnels;
pub use tunnels::Tunnels;

pub fn ready_now<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("{PENDING_RESPONSE}"),
    }
}

#[derive(Clone, Copy)]
pub enum Replacement {
    Record,
    Root,
}

pub fn replace_same_json(fixture: &Fixture, replacement: Replacement) {
    let root = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(SSH_PROFILES_DIRECTORY);
    let record = root.join(PROFILE_RECORD);
    let bytes = std::fs::read(&record).unwrap();
    let metadata = std::fs::metadata(&record).unwrap();
    match replacement {
        Replacement::Record => {
            let saved = fixture.root.path().join(SAVED_RECORD);
            std::fs::rename(&record, &saved).unwrap();
            std::fs::copy(saved, &record).unwrap();
            assert_ne!(std::fs::metadata(&record).unwrap().ino(), metadata.ino());
        }
        Replacement::Root => {
            let original = std::fs::metadata(&root).unwrap();
            let saved = fixture.root.path().join(SAVED_ROOT);
            std::fs::rename(&root, &saved).unwrap();
            std::fs::create_dir(&root).unwrap();
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(DIRECTORY_MODE))
                .unwrap();
            std::fs::copy(saved.join(PROFILE_RECORD), &record).unwrap();
            assert_ne!(std::fs::metadata(&root).unwrap().ino(), original.ino());
        }
    }
    assert_eq!(std::fs::read(record).unwrap(), bytes);
}
