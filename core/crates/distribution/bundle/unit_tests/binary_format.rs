use super::binaries::*;
use yolo_distribution_bundle::*;

#[test]
fn both_byte_orders_identify_every_supported_target() {
    for target in all_targets() {
        for little in [false, true] {
            assert_eq!(binary_target(&binary(target, little)), Some(target));
        }
        assert_eq!(
            serde_json::to_value(target.platform).unwrap(),
            target.platform.as_str()
        );
        assert_eq!(
            serde_json::to_value(target.architecture).unwrap(),
            target.architecture.as_str()
        );
    }
}

#[test]
fn truncated_unknown_and_malformed_binary_headers_are_rejected() {
    for target in all_targets() {
        let bytes = binary(target, true);
        for length in 0..bytes.len() {
            assert_eq!(binary_target(&bytes[..length]), None);
        }
        let mut invalid = bytes.clone();
        invalid[0] = INVALID_HEADER;
        assert_eq!(binary_target(&invalid), None);
        let mut invalid = bytes;
        match target.platform {
            Platform::Linux => invalid[ELF_MACHINE_OFFSET..].fill(INVALID_HEADER),
            Platform::MacOs => invalid[MACHO_CPU_OFFSET..].fill(INVALID_HEADER),
        }
        assert_eq!(binary_target(&invalid), None);
    }
    let target = Target {
        platform: Platform::Linux,
        architecture: Architecture::X86_64,
    };
    for offset in [ELF_CLASS_OFFSET, ELF_VERSION_OFFSET, ELF_ENDIAN_OFFSET] {
        let mut invalid = binary(target, true);
        invalid[offset] = INVALID_HEADER;
        assert_eq!(binary_target(&invalid), None);
    }
}
