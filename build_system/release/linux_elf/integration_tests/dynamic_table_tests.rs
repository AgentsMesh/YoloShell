use super::linux_elf_fixture::dynamic_boundaries as fixture;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::{
    ELF_MACHINE_AARCH64, LINUX_ARTIFACT_RELATIVE_PATH, READ_ONLY_EXECUTABLE_MODE,
};

fn verify(bytes: &[u8]) -> anyhow::Result<crate::VerifiedLinuxElf> {
    let root = tempfile::tempdir()?;
    let path = root.path().join(LINUX_ARTIFACT_RELATIVE_PATH);
    std::fs::write(&path, bytes)?;
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE),
    )?;
    crate::verify(&path, ELF_MACHINE_AARCH64)
}

#[test]
fn each_dynamic_metadata_field_is_mandatory_even_when_other_fields_are_valid() {
    for bytes in fixture::missing_values() {
        assert!(verify(&bytes).is_err());
    }
}

#[test]
fn dynamic_table_rejects_duplicate_segments_empty_required_tables_and_invalid_lengths() {
    let malformed = fixture::invalid_sizes()
        .into_iter()
        .chain(fixture::empty_required_tables())
        .chain([
            fixture::duplicate_segment(),
            fixture::unmapped_version_table(),
        ]);
    for bytes in malformed {
        assert!(verify(&bytes).is_err());
    }
}

#[test]
fn non_utf8_glibc_versions_fail_closed() {
    assert!(verify(&fixture::invalid_version_encoding()).is_err());
}

#[test]
fn multiple_version_requirement_records_are_traversed_and_checked() {
    verify(&fixture::two_version_records()).unwrap();
}
