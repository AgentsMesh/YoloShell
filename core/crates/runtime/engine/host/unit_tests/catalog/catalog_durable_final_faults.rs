use crate::host::tests::*;
const DURABLE_FINAL_SAVED_NAME: &str = "durable-final-saved.json";
const DURABLE_VERSION_CHANGED_DETAIL: &str = "durable state version changed";
const DURABLE_PATH_CHANGED_DETAIL: &str = "durable state path changed";
const DURABLE_FINAL_CHANGED_DETAIL: &str = "durable state changed while reading";
const DURABLE_BOUNDED_DETAIL: &str = "exceeds byte limit";
const DURABLE_FINAL_LABEL: &str = "session manifest";
const DURABLE_FINAL_FIRST_BYTE: usize = 0;
const DURABLE_FINAL_OFFSET: u64 = 0;
const DURABLE_FINAL_XOR_MASK: u8 = 1;
const DURABLE_OVERSIZE_INCREMENT: u64 = 1;

#[test]
fn durable_version_fence_rejects_replacement_after_descriptor_verification() {
    let (_directory, manifest, path, durable) = durable_final_fixture();
    let _guard = catalog_durable_file::install_durable_read_hook(
        catalog_durable_file::DurableReadStage::AfterDescriptorVerification,
        replace_durable_path,
    );

    let error = catalog_durable_file::read_optional_durable_json::<TerminalManifest>(
        &durable,
        path.file_name().unwrap(),
        DURABLE_FINAL_LABEL,
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_VERSION_CHANGED_DETAIL));
    assert!(path.with_file_name(DURABLE_FINAL_SAVED_NAME).is_file());
    assert_eq!(
        manifest.terminal_id,
        read_json::<TerminalManifest>(&path).unwrap().terminal_id
    );
}

#[test]
fn durable_path_fence_rejects_replacement_after_published_verification() {
    let (_directory, _manifest, path, durable) = durable_final_fixture();
    let _guard = catalog_durable_file::install_durable_read_hook(
        catalog_durable_file::DurableReadStage::AfterPublishedVerification,
        replace_durable_path,
    );

    let error = catalog_durable_file::read_optional_durable_json::<TerminalManifest>(
        &durable,
        path.file_name().unwrap(),
        DURABLE_FINAL_LABEL,
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_PATH_CHANGED_DETAIL));
    assert!(path.with_file_name(DURABLE_FINAL_SAVED_NAME).is_file());
}

#[test]
fn durable_content_fence_rejects_in_place_final_read_mutation() {
    use std::os::unix::fs::FileExt;

    let (_directory, _manifest, path, durable) = durable_final_fixture();
    let _guard = catalog_durable_file::install_durable_read_hook(
        catalog_durable_file::DurableReadStage::BeforeFinalRead,
        |opened| {
            let mut changed = std::fs::read(opened)?;
            changed[DURABLE_FINAL_FIRST_BYTE] ^= DURABLE_FINAL_XOR_MASK;
            let file = std::fs::OpenOptions::new().write(true).open(opened)?;
            file.write_all_at(&changed, DURABLE_FINAL_OFFSET)?;
            file.sync_data().map_err(Into::into)
        },
    );

    let error = catalog_durable_file::read_optional_durable_json::<TerminalManifest>(
        &durable,
        path.file_name().unwrap(),
        DURABLE_FINAL_LABEL,
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_FINAL_CHANGED_DETAIL));
}

#[test]
fn bounded_durable_read_rejects_growth_beyond_the_inspected_size() {
    let (_directory, _manifest, path, durable) = durable_final_fixture();
    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable,
        path.file_name().unwrap(),
        |opened| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(opened)?
                .set_len(
                    catalog_durable_file::MAX_DURABLE_JSON_BYTES + DURABLE_OVERSIZE_INCREMENT,
                )?;
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_BOUNDED_DETAIL));
}

fn durable_final_fixture() -> (
    tempfile::TempDir,
    TerminalManifest,
    PathBuf,
    catalog_durable_dir::DurableDirectory,
) {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let durable = catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();
    (directory, manifest, path, durable)
}

fn replace_durable_path(path: &Path) -> Result<()> {
    let saved = path.with_file_name(DURABLE_FINAL_SAVED_NAME);
    std::fs::rename(path, &saved)?;
    std::fs::copy(saved, path)?;
    set_private_file_mode(path);
    Ok(())
}
