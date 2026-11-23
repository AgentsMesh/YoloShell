use crate::host::tests::*;
use super::*;

#[cfg(unix)]
mod catalog_journal_safety {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::fs::symlink;

    const JOURNAL_SUFFIX: &str = "-journal";
    const JOURNAL_QUARANTINE_PREFIX: &str = "catalog.sqlite-journal.quarantine-";
    const OUTSIDE_BYTES: &[u8] = b"outside journal target";
    const SENTINEL_FILE: &str = "sentinel";

    #[test]
    fn corrupt_main_with_journal_symlink_preserves_the_target() {
        let directory = tempdir().unwrap();
        let manifest = persist_rebuild_manifest(directory.path());
        let paths = Paths::new(directory.path());
        std::fs::write(paths.catalog(), CORRUPT_CATALOG_BYTES).unwrap();
        let outside = directory.path().join("outside-journal");
        std::fs::write(&outside, OUTSIDE_BYTES).unwrap();
        symlink(&outside, journal_path(&paths)).unwrap();

        let host = Host::open(HostConfig::new(directory.path())).unwrap();

        assert_rebuilt_manifest(&host, &manifest);
        assert_eq!(std::fs::read(outside).unwrap(), OUTSIDE_BYTES);
        assert!(quarantined_journal(directory.path()).is_symlink());
    }

    #[test]
    fn missing_main_with_journal_directory_preserves_its_contents() {
        let directory = tempdir().unwrap();
        let manifest = persist_rebuild_manifest(directory.path());
        let paths = Paths::new(directory.path());
        let journal = journal_path(&paths);
        std::fs::create_dir(&journal).unwrap();
        std::fs::write(journal.join(SENTINEL_FILE), OUTSIDE_BYTES).unwrap();

        let host = Host::open(HostConfig::new(directory.path())).unwrap();

        assert_rebuilt_manifest(&host, &manifest);
        assert_eq!(
            std::fs::read(quarantined_journal(directory.path()).join(SENTINEL_FILE)).unwrap(),
            OUTSIDE_BYTES
        );
    }

    #[test]
    fn missing_main_with_journal_hard_link_preserves_the_other_name() {
        let directory = tempdir().unwrap();
        let manifest = persist_rebuild_manifest(directory.path());
        let paths = Paths::new(directory.path());
        let outside = directory.path().join("outside-journal");
        std::fs::write(&outside, OUTSIDE_BYTES).unwrap();
        std::fs::hard_link(&outside, journal_path(&paths)).unwrap();

        let host = Host::open(HostConfig::new(directory.path())).unwrap();

        assert_rebuilt_manifest(&host, &manifest);
        assert_eq!(std::fs::read(outside).unwrap(), OUTSIDE_BYTES);
        assert!(quarantined_journal(directory.path()).is_file());
    }

    fn journal_path(paths: &Paths) -> PathBuf {
        let mut path: OsString = paths.catalog().into_os_string();
        path.push(JOURNAL_SUFFIX);
        PathBuf::from(path)
    }

    fn quarantined_journal(home: &Path) -> PathBuf {
        std::fs::read_dir(home)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(JOURNAL_QUARANTINE_PREFIX)
            })
            .unwrap()
            .path()
    }
}
