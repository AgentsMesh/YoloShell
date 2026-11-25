use super::retained_test_fixture::{RetainedFixture, SENTINEL_BYTES, SHM_SUFFIX};
use super::secure_vfs_file::ArtifactKind;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ALIAS_DIRECTORY_MODE: u32 = 0o700;
const MOVED_ALIAS_PREFIX: &str = ".moved-yolo-sqlite-alias-";

struct AliasReplacement {
    alias: PathBuf,
    moved: PathBuf,
}

impl AliasReplacement {
    fn install(alias: &Path) -> anyhow::Result<Self> {
        let parent = alias
            .parent()
            .ok_or_else(|| anyhow::anyhow!("SHM alias has no parent"))?;
        let moved = parent.with_file_name(format!("{MOVED_ALIAS_PREFIX}{}", uuid::Uuid::new_v4()));
        std::fs::rename(parent, &moved)?;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(ALIAS_DIRECTORY_MODE).create(parent)?;
        std::fs::write(alias, SENTINEL_BYTES)?;
        Ok(Self {
            alias: alias.to_path_buf(),
            moved,
        })
    }

    fn assert_untouched(&self) {
        assert_eq!(std::fs::read(&self.alias).unwrap(), SENTINEL_BYTES);
    }

    fn cleanup(self) {
        std::fs::remove_dir_all(self.alias.parent().unwrap()).unwrap();
        std::fs::remove_dir_all(self.moved).unwrap();
    }
}

#[test]
fn shm_map_cleanup_never_deletes_an_alias_directory_replacement() {
    let fixture = RetainedFixture::new();
    let replacement = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&replacement);
    super::secure_vfs_tests::set_alias(ArtifactKind::Shm, move |alias| {
        *observed.lock().unwrap() = Some(AliasReplacement::install(alias)?);
        Ok(())
    });

    let result = super::Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new()));

    drop(result);
    let replacement = replacement.lock().unwrap().take().unwrap();
    replacement.assert_untouched();
    replacement.cleanup();
}

#[test]
fn clean_close_deletes_retained_shm_without_touching_an_alias_replacement() {
    let fixture = RetainedFixture::new();
    let alias = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&alias);
    super::secure_vfs_tests::set_alias(ArtifactKind::Shm, move |path| {
        *observed.lock().unwrap() = Some(path.to_path_buf());
        Ok(())
    });
    let (catalog, _) =
        super::Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    let replacement = AliasReplacement::install(&alias.lock().unwrap().take().unwrap()).unwrap();
    let shm = fixture.sidecar(SHM_SUFFIX);
    assert!(shm.exists());

    drop(catalog);

    replacement.assert_untouched();
    assert!(!shm.exists());
    replacement.cleanup();
}

#[test]
fn old_generation_close_preserves_shm_for_current_and_third_connections() {
    let fixture = RetainedFixture::new();
    let (old_generation, _) =
        super::Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    let current_generation = super::Catalog::open(&fixture.catalog).unwrap();
    let shm = fixture.sidecar(SHM_SUFFIX);
    let identity = std::fs::metadata(&shm).unwrap().ino();

    drop(old_generation);
    assert_eq!(std::fs::metadata(&shm).unwrap().ino(), identity);
    let third_connection = super::Catalog::open(&fixture.catalog).unwrap();
    assert_eq!(std::fs::metadata(&shm).unwrap().ino(), identity);
    current_generation.integrity_check().unwrap();
    drop(current_generation);
    assert_eq!(std::fs::metadata(&shm).unwrap().ino(), identity);
    third_connection.integrity_check().unwrap();
    drop(third_connection);

    assert!(!shm.exists());
}
