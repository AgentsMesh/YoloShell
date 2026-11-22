use super::linux_elf_fixture::{corruption::*, versioned_elf};
use yolo_release_contracts::{
    ELF_MACHINE_X86_64, GLIBC_SYMBOL_VERSION_PREFIX, LINUX_MINIMUM_GLIBC_VERSION,
};

const STRUCTURE_ERROR: &str = "Linux runtime ELF";
const INVALID_VERSIONS: &[&str] = &["2", "2.31.0.0.0", "2.-1", "2.4294967296"];
const VALID_EXTENDED_VERSIONS: &[&str] = &["2.31.0", "2.31.0.0", "2.30.99", "1.100"];

#[test]
fn malformed_elf_offsets_counts_and_linked_records_fail_closed() {
    let baseline = format!("{GLIBC_SYMBOL_VERSION_PREFIX}{LINUX_MINIMUM_GLIBC_VERSION}");
    let corruptions: &[fn(&mut [u8])] = &[
        invalid_program_entry_size,
        too_many_program_headers,
        string_address_before_load_segment,
        string_address_beyond_load_segment,
        too_many_version_needs,
        invalid_version_revision,
        empty_auxiliary_list,
        too_many_auxiliaries,
        truncated_auxiliary_pointer,
        truncated_version_name_pointer,
        unterminated_version_name,
        nonterminal_last_version_record,
        nonterminal_last_auxiliary_record,
        overlapping_auxiliary_records,
        duplicate_dynamic_value,
        unterminated_dynamic_table,
    ];
    for (index, corrupt) in corruptions.iter().enumerate() {
        let mut bytes = versioned_elf(ELF_MACHINE_X86_64, &[&baseline]);
        corrupt(&mut bytes);
        let error = crate::glibc::verify(&bytes).unwrap_err().to_string();
        assert!(
            error.starts_with(STRUCTURE_ERROR),
            "mutation {index}: {error}"
        );
    }
}

#[test]
fn glibc_versions_require_bounded_numeric_components() {
    for version in INVALID_VERSIONS {
        let requirement = format!("{GLIBC_SYMBOL_VERSION_PREFIX}{version}");
        let bytes = versioned_elf(ELF_MACHINE_X86_64, &[&requirement]);
        assert_eq!(
            crate::glibc::verify(&bytes).unwrap_err().to_string(),
            "Linux runtime GLIBC version name is invalid"
        );
    }
    for version in VALID_EXTENDED_VERSIONS {
        let requirement = format!("{GLIBC_SYMBOL_VERSION_PREFIX}{version}");
        let bytes = versioned_elf(ELF_MACHINE_X86_64, &[&requirement]);
        crate::glibc::verify(&bytes).unwrap();
    }
}
