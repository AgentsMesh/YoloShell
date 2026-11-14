use crate::host::tests::*;
use super::*;

const DURABLE_IN_PLACE_CHANGED_MESSAGE: &str = "durable state changed while reading";
const DURABLE_IN_PLACE_FIRST_BYTE: usize = 0;
const DURABLE_IN_PLACE_OFFSET: u64 = 0;
const DURABLE_IN_PLACE_XOR_MASK: u8 = 1;

#[test]
fn durable_manifest_rejects_equal_length_rewrite_and_restore() {
    use std::fs::OpenOptions;
    use std::os::unix::fs::FileExt;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let bytes = std::fs::read(&path).unwrap();
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |opened_path| {
            let file = OpenOptions::new().write(true).open(opened_path)?;
            let mut changed = bytes.clone();
            changed[DURABLE_IN_PLACE_FIRST_BYTE] ^= DURABLE_IN_PLACE_XOR_MASK;
            file.write_all_at(&changed, DURABLE_IN_PLACE_OFFSET)?;
            file.sync_data()?;
            file.write_all_at(&bytes, DURABLE_IN_PLACE_OFFSET)?;
            file.sync_data()?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_IN_PLACE_CHANGED_MESSAGE));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn durable_manifest_rejects_write_after_published_version_check() {
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{FileExt, MetadataExt};

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let mut changed = std::fs::read(&path).unwrap();
    changed[DURABLE_IN_PLACE_FIRST_BYTE] ^= DURABLE_IN_PLACE_XOR_MASK;
    let metadata = path.metadata().unwrap();
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_final_hook(
        &durable_directory,
        path.file_name().unwrap(),
        |opened_path| {
            let file = OpenOptions::new().write(true).open(opened_path)?;
            file.write_all_at(&changed, DURABLE_IN_PLACE_OFFSET)?;
            let times = [
                libc::timespec {
                    tv_sec: metadata.atime(),
                    tv_nsec: metadata.atime_nsec(),
                },
                libc::timespec {
                    tv_sec: metadata.mtime(),
                    tv_nsec: metadata.mtime_nsec(),
                },
            ];
            anyhow::ensure!(unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) } == 0);
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_IN_PLACE_CHANGED_MESSAGE));
    assert_eq!(std::fs::read(path).unwrap(), changed);
}
