use super::*;
use std::os::unix::ffi::OsStringExt;
use yolo_architecture_test_support::Fixture;

#[test]
fn resolution_precedence_is_override_test_runfiles_manifest_build_then_ancestor() {
    let fixture = Fixture::new();
    for relative in [
        "explicit",
        "test/main",
        "run/main",
        "snapshot",
        "build",
        "current/nested",
    ] {
        fs::create_dir_all(fixture.root().join(relative)).unwrap();
    }
    fixture.write(&format!("current/{WORKSPACE_MARKER}"), "");
    let manifest = fixture.root().join("MANIFEST");
    fs::write(
        &manifest,
        format!(
            "main/{WORKSPACE_MARKER} {}\n",
            fixture
                .root()
                .join("snapshot")
                .join(WORKSPACE_MARKER)
                .display()
        ),
    )
    .unwrap();
    let mut values = Locations {
        override_root: Some(fixture.root().join("explicit").into_os_string()),
        test_source: Some(fixture.root().join("test").into_os_string()),
        workspace: Some("main".into()),
        runfiles: Some(fixture.root().join("run").into_os_string()),
        manifest: Some(manifest.into_os_string()),
        build_root: Some(fixture.root().join("build").into_os_string()),
        executable: None,
    };
    let current = fixture.root().join("current/nested");
    let check = |values: &Locations, expected: &str| {
        assert_eq!(
            locate_with(values, &current).unwrap(),
            fixture.root().join(expected).canonicalize().unwrap()
        );
    };
    check(&values, "explicit");
    values.override_root = None;
    check(&values, "test/main");
    values.test_source = Some(fixture.root().join("missing").into_os_string());
    check(&values, "run/main");
    values.runfiles = Some(fixture.root().join("missing").into_os_string());
    check(&values, "snapshot");
    values.manifest = None;
    check(&values, "build");
    values.build_root = None;
    check(&values, "current");
}

#[test]
fn invalid_explicit_roots_and_unreadable_manifests_do_not_fall_back() {
    let fixture = Fixture::new();
    fixture.write(WORKSPACE_MARKER, "");
    fixture.write("regular", "");
    for relative in ["missing", "regular"] {
        let root = fixture.root().join(relative);
        assert_eq!(
            locate(Some(&root)).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        let values = Locations {
            override_root: Some(root.into_os_string()),
            ..Locations::default()
        };
        assert_eq!(
            locate_with(&values, fixture.root()).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
    let values = Locations {
        manifest: Some(fixture.root().join("absent-manifest").into_os_string()),
        build_root: Some(fixture.root().as_os_str().to_owned()),
        ..Locations::default()
    };
    assert!(locate_with(&values, fixture.root()).is_err());
    assert_eq!(
        locate(Some(fixture.root())).unwrap(),
        fixture.root().canonicalize().unwrap()
    );
}

#[test]
fn runfiles_overlay_never_uses_live_workspace_fallbacks() {
    let fixture = Fixture::new();
    let mut values = Locations {
        override_root: Some(fixture.root().as_os_str().to_owned()),
        build_root: Some(fixture.root().as_os_str().to_owned()),
        ..Locations::default()
    };
    assert_eq!(runfiles_with(&values).unwrap(), None);
    let repository = BAZEL_MAIN_REPOSITORY_DIRECTORY;
    let base = fixture.root().join("run");
    values.runfiles = Some(base.clone().into_os_string());
    assert_eq!(runfiles_with(&values).unwrap(), None);
    fs::create_dir_all(base.join(repository)).unwrap();
    assert_eq!(
        runfiles_with(&values).unwrap(),
        Some(base.join(repository).canonicalize().unwrap())
    );
    values.test_source = Some(fixture.root().join("test").into_os_string());
    values.workspace = Some(repository.into());
    assert_eq!(
        runfiles_with(&values).unwrap(),
        Some(base.join(repository).canonicalize().unwrap())
    );
    let test_root = fixture.root().join("test").join(repository);
    fs::create_dir_all(&test_root).unwrap();
    assert_eq!(
        runfiles_with(&values).unwrap(),
        Some(test_root.canonicalize().unwrap())
    );
}

#[test]
fn adjacent_runfiles_and_manifest_edges_remain_bounded() {
    assert!(adjacent_runfiles(None).is_none());
    assert!(adjacent_runfiles(Some(Path::new("/"))).is_none());
    let invalid = PathBuf::from(OsString::from_vec(vec![u8::MAX]));
    assert!(adjacent_runfiles(Some(&invalid)).is_none());
    let fixture = Fixture::new();
    let manifest = fixture.root().join("MANIFEST");
    for contents in [
        "no-separator\nother/key ignored\n".to_owned(),
        format!("main/{WORKSPACE_MARKER} /\n"),
    ] {
        fs::write(&manifest, contents).unwrap();
        assert_eq!(
            root_from_manifest(&manifest, OsStr::new("main")).unwrap(),
            None
        );
    }
    assert!(joined_workspace(None, Some(OsStr::new("main"))).is_none());
    assert!(joined_workspace(Some(&OsString::from("base")), None).is_none());
}
