use super::*;

fn entries(root: &std::path::Path) -> Vec<std::ffi::OsString> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

#[test]
fn fixture_home_never_creates_a_wrapper_in_the_custody_root() {
    let mut socket_root = crate::paths::IsolatedTestSocketRoot::create().unwrap();
    let before = entries(socket_root.path());

    let fixture = FixtureFilesystem::new_with_socket_root(socket_root.path().to_path_buf());
    let after = entries(socket_root.path());

    assert_eq!(before, after);
    assert!(!fixture.home_root.path().starts_with(socket_root.path()));
    assert_eq!(
        fixture.paths.socket_root_override_for_test(),
        Some(
            crate::paths::platform::platform_path(&visible_socket_root(socket_root.path())).as_path(),
        )
    );
    socket_root.finalize().unwrap();
}

#[test]
fn direct_fixture_namespace_remains_scannable_after_home_outlives_it() {
    let mut socket_root = crate::paths::IsolatedTestSocketRoot::create().unwrap();
    let fixture = FixtureFilesystem::new_with_socket_root(socket_root.path().to_path_buf());
    let retained = fixture.paths.retain_socket_namespace().unwrap();
    let namespace = retained.directory().unwrap().path().to_path_buf();
    let visible_root = visible_socket_root(socket_root.path());
    let namespace_name = namespace.file_name().unwrap().to_owned();

    let created = entries(socket_root.path());
    let canonical_visible_root = crate::paths::platform::platform_path(&visible_root);
    assert_eq!(namespace.parent(), Some(canonical_visible_root.as_path()));
    assert!(created.contains(&namespace_name));
    assert!(
        created
            .iter()
            .all(|name| !name.to_string_lossy().starts_with(FIXTURE_ROOT_PREFIX))
    );

    drop(retained);
    socket_root.finalize().unwrap();
    assert!(fixture.home_root.path().exists());
}
