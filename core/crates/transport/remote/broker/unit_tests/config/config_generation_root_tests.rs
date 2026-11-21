use super::*;
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const APPEARING_GENERATION_NAME: &str = "appearing-generation";
const MISSING_GENERATION_NAME: &str = "confirmed-missing-generation";
const REPLACEMENT_DIRECTORY_NAME: &str = "replacement-generations";

#[test]
fn generation_appearing_after_first_absence_is_opened() {
    let fixture = RootFixture::new();
    let name = entry_name(APPEARING_GENERATION_NAME).unwrap();
    let path = fixture.root_path.join(APPEARING_GENERATION_NAME);

    let opened = fixture
        .root
        .open_generation_with_absence_hook(&name, || {
            fs::create_dir(&path)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE))?;
            Ok(())
        })
        .unwrap();

    assert!(opened.is_some());
}

#[test]
fn missing_generation_is_confirmed_twice() {
    let fixture = RootFixture::new();
    let name = entry_name(MISSING_GENERATION_NAME).unwrap();
    let hook_called = Cell::new(false);

    let opened = fixture
        .root
        .open_generation_with_absence_hook(&name, || {
            hook_called.set(true);
            Ok(())
        })
        .unwrap();

    assert!(hook_called.get());
    assert!(opened.is_none());
}

#[test]
fn root_replacement_during_absence_confirmation_fails_closed() {
    let fixture = RootFixture::new();
    let name = entry_name(MISSING_GENERATION_NAME).unwrap();
    let displaced = fixture.base.path().join(REPLACEMENT_DIRECTORY_NAME);

    let result = fixture.root.open_generation_with_absence_hook(&name, || {
        fs::rename(&fixture.root_path, &displaced)?;
        fs::create_dir(&fixture.root_path)?;
        Ok(())
    });

    assert!(result.is_err());
    assert!(displaced.is_dir());
}

struct RootFixture {
    base: tempfile::TempDir,
    root: GenerationRoot,
    root_path: std::path::PathBuf,
}

impl RootFixture {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
        let root = GenerationRoot::ensure(&root_path).unwrap();
        Self {
            base,
            root,
            root_path,
        }
    }
}
