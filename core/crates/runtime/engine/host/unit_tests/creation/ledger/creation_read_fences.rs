use crate::host::tests::*;
use super::*;

use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt as ReadFenceDirBuilderExt, FileExt as ReadFenceFileExt};

const READ_FENCE_FILE_OFFSET: u64 = 0;
const READ_FENCE_FIRST_BYTE: usize = 0;
const READ_FENCE_XOR_MASK: u8 = 1;
const READ_FENCE_WRITABLE_MODE: u32 = 0o600;
const SYSCALL_SUCCESS: libc::c_int = 0;
const LEDGER_HOME_NAME: &str = "ledger-home";
const MOVED_LEDGER_PARENT_NAME: &str = "moved-ledger-home";
const MOVED_RECEIPT_INDEX_NAME: &str = "moved-receipt-index";
const MTIME_RESTORE_ERROR: &str = "test mutation did not restore mtime";

#[test]
fn missing_ledger_rejects_parent_replacement_with_a_new_visible_ledger() {
    let root = tempdir().unwrap();
    let home = root.path().join(LEDGER_HOME_NAME);
    crate::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let path = home.join(CREATION_LEDGER_FILE_NAME);
    let moved = root.path().join(MOVED_LEDGER_PARENT_NAME);
    let encoded = encode_creation_ledger(&CreationLedger::default(), &path).unwrap();

    let result = read_creation_ledger_with_test_hook(&path, |stage| {
        if stage == CreationDurableReadStage::MissingObserved {
            fs::rename(&home, &moved)?;
            create_private_test_directory(&home)?;
            fs::write(&path, &encoded)?;
            fs::set_permissions(&path, private_test_permissions(CREATION_LEDGER_FILE_MODE))?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), encoded);
    assert!(!moved.join(CREATION_LEDGER_FILE_NAME).exists());
}

#[test]
fn ledger_appearing_between_absence_checks_is_read() {
    let root = tempdir().unwrap();
    Paths::new(root.path()).ensure_base().unwrap();
    let path = root.path().join(CREATION_LEDGER_FILE_NAME);
    let expected = CreationLedger::default();

    let loaded = read_creation_ledger_with_test_hook(&path, |stage| {
        if stage == CreationDurableReadStage::MissingObserved {
            write_creation_ledger(&path, &expected)?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(loaded, Some(expected));
}

#[test]
fn ledger_read_rejects_equal_length_rewrite_with_restored_bytes_and_mtime() {
    let root = tempdir().unwrap();
    Paths::new(root.path()).ensure_base().unwrap();
    let path = root.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let original = fs::read(&path).unwrap();

    let result = read_creation_ledger_with_test_hook(&path, |stage| {
        if stage == CreationDurableReadStage::AfterRead {
            tamper_and_restore(&path, &original, CREATION_LEDGER_FILE_MODE)?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn missing_receipt_rejects_index_replacement_with_a_new_visible_receipt() {
    let (root, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let encoded = serde_json::to_vec(&receipt).unwrap();
    let name = binding.creation_token_digest.as_str();
    let target = store.token_index.path.join(name);
    let moved = root.path().join(MOVED_RECEIPT_INDEX_NAME);

    let result = read_receipt_at_with_test_hook(&store.token_index, name, |stage| {
        if stage == ReceiptReadStage::MissingObserved {
            fs::rename(&store.token_index.path, &moved)?;
            create_private_test_directory(&store.token_index.path)?;
            fs::write(&target, &encoded)?;
            fs::set_permissions(
                &target,
                private_test_permissions(CREATION_RECEIPT_FILE_MODE),
            )?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(target).unwrap(), encoded);
    assert!(!moved.join(name).exists());
}

#[test]
fn receipt_appearing_between_absence_checks_is_read() {
    let (_root, store, binding, _) = receipt_test_setup();
    let expected = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let name = binding.creation_token_digest.as_str();

    let loaded = read_receipt_at_with_test_hook(&store.token_index, name, |stage| {
        if stage == ReceiptReadStage::MissingObserved {
            publish_receipt_at(&store.staging, &store.token_index, name, &expected)?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(loaded, Some(expected));
}

#[test]
fn receipt_read_rejects_equal_length_rewrite_with_restored_bytes_and_mtime() {
    let (_root, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Archived);
    let name = binding.creation_token_digest.as_str();
    publish_receipt_at(&store.staging, &store.token_index, name, &receipt).unwrap();
    let path = store.token_index.path.join(name);
    let original = fs::read(&path).unwrap();

    let result = read_receipt_at_with_test_hook(&store.token_index, name, |stage| {
        if stage == ReceiptReadStage::AfterRead {
            tamper_and_restore(&path, &original, CREATION_RECEIPT_FILE_MODE)?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), original);
}

fn create_private_test_directory(path: &Path) -> std::io::Result<()> {
    fs::DirBuilder::new()
        .mode(crate::host::TEST_PRIVATE_DIRECTORY_MODE)
        .create(path)
}

fn tamper_and_restore(path: &Path, original: &[u8], final_mode: u32) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mut changed = original.to_vec();
    changed[READ_FENCE_FIRST_BYTE] ^= READ_FENCE_XOR_MASK;
    if final_mode != READ_FENCE_WRITABLE_MODE {
        fs::set_permissions(path, private_test_permissions(READ_FENCE_WRITABLE_MODE))?;
    }
    let file = OpenOptions::new().write(true).open(path)?;
    file.write_all_at(&changed, READ_FENCE_FILE_OFFSET)?;
    file.sync_data()?;
    file.write_all_at(original, READ_FENCE_FILE_OFFSET)?;
    file.sync_data()?;
    restore_file_times(&file, &metadata)?;
    if final_mode != READ_FENCE_WRITABLE_MODE {
        fs::set_permissions(path, private_test_permissions(final_mode))?;
    }
    anyhow::ensure!(
        same_mtime(&metadata, &fs::metadata(path)?),
        MTIME_RESTORE_ERROR
    );
    Ok(())
}

fn restore_file_times(file: &fs::File, metadata: &fs::Metadata) -> std::io::Result<()> {
    let times = [
        libc::timespec {
            tv_sec: std::os::unix::fs::MetadataExt::atime(metadata),
            tv_nsec: std::os::unix::fs::MetadataExt::atime_nsec(metadata),
        },
        libc::timespec {
            tv_sec: std::os::unix::fs::MetadataExt::mtime(metadata),
            tv_nsec: std::os::unix::fs::MetadataExt::mtime_nsec(metadata),
        },
    ];
    if unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) } == SYSCALL_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn private_test_permissions(mode: u32) -> fs::Permissions {
    <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(mode)
}

fn same_mtime(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    std::os::unix::fs::MetadataExt::mtime(left) == std::os::unix::fs::MetadataExt::mtime(right)
        && std::os::unix::fs::MetadataExt::mtime_nsec(left)
            == std::os::unix::fs::MetadataExt::mtime_nsec(right)
}
