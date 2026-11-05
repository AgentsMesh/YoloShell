use super::*;
use crate::paths::Paths;

#[test]
fn retained_namespace_is_released_only_after_an_idle_settlement_pause() {
    let home = tempfile::tempdir().unwrap();
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let paths = Paths::new_with_socket_root(home.path(), path.clone());
    let mut namespace = Some(paths.retain_socket_namespace().unwrap());
    let namespace_path = paths.socket_dir().unwrap();
    let mut pauses = Vec::new();

    let result = settle_with_pause(&mut root, &mut |duration| {
        assert!(path.is_dir());
        assert!(namespace_path.is_dir());
        namespace.as_ref().unwrap().verify().unwrap();
        pauses.push(duration);
        drop(namespace.take());
    });
    let released_during_pause = namespace.is_none();
    drop(namespace);
    root.finalize().unwrap();

    result.unwrap();
    assert!(released_during_pause);
    assert_eq!(
        pauses,
        [Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS)]
    );
    assert!(!path.exists());
    assert!(!namespace_path.exists());
}
