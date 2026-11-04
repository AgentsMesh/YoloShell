use super::linux_elf_fixture::{
    overflow_string_table, remove_version_metadata, static_elf, versioned_elf,
};
use crate as linux_elf;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::*;

const OLDER_GLIBC_VERSION: &str = "2.9";
const NEWER_GLIBC_VERSION: &str = "2.32";
const LEXICALLY_SMALLER_UNSUPPORTED_VERSION: &str = "2.100";
const NON_NUMERIC_GLIBC_VERSION: &str = "PRIVATE";
const NON_GLIBC_VERSION: &str = "GCC_4.2.0";

#[test]
fn exact_elf_machine_and_digest_are_verified() {
    let fixture = ElfFixture::new(ELF_MACHINE_X86_64);
    let verified = linux_elf::verify(&fixture.path, ELF_MACHINE_X86_64).unwrap();
    assert_eq!(
        verified.sha256,
        yolo_release_foundation::digest::bytes_sha256(&fixture.bytes)
    );
    assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_AARCH64).is_err());
}

#[test]
fn structured_glibc_requirements_accept_the_supported_numeric_range() {
    let older = glibc(OLDER_GLIBC_VERSION);
    let baseline = glibc(LINUX_MINIMUM_GLIBC_VERSION);
    let fixture = ElfFixture::from_bytes(versioned_elf(
        ELF_MACHINE_X86_64,
        &[NON_GLIBC_VERSION, &older, &baseline],
    ));
    linux_elf::verify(&fixture.path, ELF_MACHINE_X86_64).unwrap();
}

#[test]
fn newer_glibc_requirements_are_compared_numerically_and_rejected() {
    for version in [NEWER_GLIBC_VERSION, LEXICALLY_SMALLER_UNSUPPORTED_VERSION] {
        let requirement = glibc(version);
        let fixture =
            ElfFixture::from_bytes(versioned_elf(ELF_MACHINE_AARCH64, &[requirement.as_str()]));
        assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_AARCH64).is_err());
    }
}

#[test]
fn dynamic_elfs_fail_closed_without_valid_glibc_version_needs() {
    for version in [
        NON_GLIBC_VERSION.to_owned(),
        glibc(NON_NUMERIC_GLIBC_VERSION),
    ] {
        let fixture =
            ElfFixture::from_bytes(versioned_elf(ELF_MACHINE_X86_64, &[version.as_str()]));
        assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_X86_64).is_err());
    }
    let baseline = glibc(LINUX_MINIMUM_GLIBC_VERSION);
    let mut truncated = versioned_elf(ELF_MACHINE_X86_64, &[&baseline]);
    truncated.pop();
    let fixture = ElfFixture::from_bytes(truncated);
    assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_X86_64).is_err());

    for corrupt in [
        remove_version_metadata as fn(&mut [u8]),
        overflow_string_table,
    ] {
        let mut bytes = versioned_elf(ELF_MACHINE_X86_64, &[&baseline]);
        corrupt(&mut bytes);
        let fixture = ElfFixture::from_bytes(bytes);
        assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_X86_64).is_err());
    }
}

#[test]
fn malformed_or_non_executable_artifacts_fail_closed() {
    let mut fixture = ElfFixture::new(ELF_MACHINE_AARCH64);
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_AARCH64).is_err());
    fs::set_permissions(
        &fixture.path,
        fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE),
    )
    .unwrap();
    for (offset, invalid) in [
        (0, 0),
        (ELF_CLASS_OFFSET, ELF_64_CLASS.saturating_add(1)),
        (ELF_DATA_OFFSET, ELF_LITTLE_ENDIAN_DATA.saturating_add(1)),
    ] {
        let original = fixture.bytes[offset];
        fixture.bytes[offset] = invalid;
        fs::set_permissions(&fixture.path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        fs::write(&fixture.path, &fixture.bytes).unwrap();
        fs::set_permissions(
            &fixture.path,
            fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE),
        )
        .unwrap();
        assert!(linux_elf::verify(&fixture.path, ELF_MACHINE_AARCH64).is_err());
        fixture.bytes[offset] = original;
    }
}

fn glibc(version: &str) -> String {
    format!("{GLIBC_SYMBOL_VERSION_PREFIX}{version}")
}

struct ElfFixture {
    _root: tempfile::TempDir,
    path: std::path::PathBuf,
    bytes: Vec<u8>,
}

impl ElfFixture {
    fn new(machine: u16) -> Self {
        Self::from_bytes(static_elf(machine))
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(LINUX_ARTIFACT_RELATIVE_PATH);
        fs::write(&path, &bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE)).unwrap();
        Self {
            _root: root,
            path,
            bytes,
        }
    }
}
