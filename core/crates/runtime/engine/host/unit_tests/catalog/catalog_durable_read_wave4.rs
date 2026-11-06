use crate::host::tests::*;
const DURABLE_MATRIX_CORRUPT_JSON: &[u8] = b"{invalid-durable-json";
const DURABLE_MATRIX_HOOK_ERROR: &str = "durable matrix hook failure";
const DURABLE_MATRIX_OVERSIZE_INCREMENT: usize = 1;
const DURABLE_MATRIX_SAVED_NAME: &str = "durable-matrix-saved.json";
const DURABLE_MATRIX_FIRST_BYTE: usize = 0;
const DURABLE_MATRIX_OFFSET: u64 = 0;
const DURABLE_MATRIX_XOR_MASK: u8 = 1;
const DURABLE_MATRIX_GROWTH_BYTE: u8 = b' ';
const DURABLE_MATRIX_GROWTH_BYTES: &[u8] = b" ";
const DURABLE_MATRIX_MINIMAL_JSON: &[u8] = b"{}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableReadFault {
    None,
    Missing,
    InvalidJson,
    Oversized,
    MetadataHook,
    MetadataGrowth,
    MetadataRename,
    MetadataMode,
    ReadHook,
    ReadGrowth,
    ReadRemoval,
    ReadReplacement,
}

#[test]
fn durable_read_faults_share_one_instrumented_instance() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    for fault in [
        DurableReadFault::None,
        DurableReadFault::Missing,
        DurableReadFault::InvalidJson,
        DurableReadFault::Oversized,
        DurableReadFault::MetadataHook,
        DurableReadFault::MetadataGrowth,
        DurableReadFault::MetadataRename,
        DurableReadFault::MetadataMode,
        DurableReadFault::ReadHook,
        DurableReadFault::ReadGrowth,
        DurableReadFault::ReadRemoval,
        DurableReadFault::ReadReplacement,
    ] {
        let directory = tempdir().unwrap();
        let manifest = persist_rebuild_manifest(directory.path());
        let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
        let saved = directory.path().join(DURABLE_MATRIX_SAVED_NAME);
        match fault {
            DurableReadFault::Missing => std::fs::remove_file(&path).unwrap(),
            DurableReadFault::InvalidJson => {
                write_private_bytes(&path, DURABLE_MATRIX_CORRUPT_JSON)
            }
            DurableReadFault::Oversized => std::fs::write(
                &path,
                vec![
                    DURABLE_MATRIX_GROWTH_BYTE;
                    usize::try_from(catalog_durable_file::MAX_DURABLE_JSON_BYTES).unwrap()
                        + DURABLE_MATRIX_OVERSIZE_INCREMENT
                ],
            )
            .unwrap(),
            _ => {}
        }
        let durable = catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();
        let result = catalog_durable_file::read_manifest_with_hooks(
            &durable,
            path.file_name().unwrap(),
            |opened| match fault {
                DurableReadFault::MetadataHook => anyhow::bail!(DURABLE_MATRIX_HOOK_ERROR),
                DurableReadFault::MetadataGrowth => {
                    std::fs::OpenOptions::new()
                        .append(true)
                        .open(opened)?
                        .write_all(DURABLE_MATRIX_GROWTH_BYTES)?;
                    Ok(())
                }
                DurableReadFault::MetadataRename => {
                    std::fs::rename(opened, &saved).map_err(Into::into)
                }
                DurableReadFault::MetadataMode => std::fs::set_permissions(
                    opened,
                    std::fs::Permissions::from_mode(PUBLIC_DURABLE_FILE_MODE),
                )
                .map_err(Into::into),
                _ => Ok(()),
            },
            |opened| match fault {
                DurableReadFault::ReadHook => anyhow::bail!(DURABLE_MATRIX_HOOK_ERROR),
                DurableReadFault::ReadGrowth => {
                    std::fs::OpenOptions::new()
                        .append(true)
                        .open(opened)?
                        .write_all(DURABLE_MATRIX_GROWTH_BYTES)?;
                    Ok(())
                }
                DurableReadFault::ReadRemoval => std::fs::remove_file(opened).map_err(Into::into),
                DurableReadFault::ReadReplacement => {
                    std::fs::rename(opened, &saved)?;
                    std::fs::copy(&saved, opened)?;
                    std::fs::set_permissions(
                        opened,
                        std::fs::Permissions::from_mode(PRIVATE_MANIFEST_MODE),
                    )?;
                    Ok(())
                }
                _ => Ok(()),
            },
        );
        match fault {
            DurableReadFault::None => {
                assert_eq!(result.unwrap().unwrap().terminal_id, manifest.terminal_id)
            }
            DurableReadFault::Missing => assert!(result.unwrap().is_none()),
            _ => assert!(result.is_err(), "fault unexpectedly succeeded: {fault:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableFinalFault {
    None,
    Remove,
    Rename,
    Mode,
    Replace,
    Rewrite,
}

#[test]
fn durable_final_verification_rejects_every_real_path_change() {
    use std::os::unix::fs::{FileExt, PermissionsExt};
    for fault in [
        DurableFinalFault::None,
        DurableFinalFault::Remove,
        DurableFinalFault::Rename,
        DurableFinalFault::Mode,
        DurableFinalFault::Replace,
        DurableFinalFault::Rewrite,
    ] {
        let directory = tempdir().unwrap();
        let manifest = persist_rebuild_manifest(directory.path());
        let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
        let saved = directory.path().join(DURABLE_MATRIX_SAVED_NAME);
        let mut changed = std::fs::read(&path).unwrap();
        changed[DURABLE_MATRIX_FIRST_BYTE] ^= DURABLE_MATRIX_XOR_MASK;
        let durable = catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();
        let result = catalog_durable_file::read_manifest_with_final_hook(
            &durable,
            path.file_name().unwrap(),
            |opened| match fault {
                DurableFinalFault::None => Ok(()),
                DurableFinalFault::Remove => std::fs::remove_file(opened).map_err(Into::into),
                DurableFinalFault::Rename => std::fs::rename(opened, &saved).map_err(Into::into),
                DurableFinalFault::Mode => std::fs::set_permissions(
                    opened,
                    std::fs::Permissions::from_mode(PUBLIC_DURABLE_FILE_MODE),
                )
                .map_err(Into::into),
                DurableFinalFault::Replace => {
                    std::fs::rename(opened, &saved)?;
                    std::fs::copy(&saved, opened)?;
                    std::fs::set_permissions(
                        opened,
                        std::fs::Permissions::from_mode(PRIVATE_MANIFEST_MODE),
                    )?;
                    Ok(())
                }
                DurableFinalFault::Rewrite => {
                    let file = std::fs::OpenOptions::new().write(true).open(opened)?;
                    file.write_all_at(&changed, DURABLE_MATRIX_OFFSET)?;
                    file.sync_data().map_err(Into::into)
                }
            },
        );
        if fault == DurableFinalFault::None {
            assert_eq!(result.unwrap().unwrap().terminal_id, manifest.terminal_id);
        } else {
            assert!(result.is_err(), "fault unexpectedly succeeded: {fault:?}");
        }
    }
}
