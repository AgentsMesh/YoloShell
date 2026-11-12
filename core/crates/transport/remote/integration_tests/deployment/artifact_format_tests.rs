use super::artifact_format::binary_target;
use super::*;
use crate::remote::constants::*;

#[test]
fn elf_endianness_and_architectures_map_to_linux() {
    for (machine, architecture) in [
        (ELF_MACHINE_X86_64, RemoteArchitecture::X86_64),
        (ELF_MACHINE_AARCH64, RemoteArchitecture::Aarch64),
    ] {
        for endian in [ELF_LITTLE_ENDIAN, ELF_BIG_ENDIAN] {
            let bytes = elf(machine, endian);
            let target = binary_target(&bytes).unwrap();
            assert_eq!(target.platform, RemotePlatform::Linux);
            assert_eq!(target.architecture, architecture);
        }
    }
}

#[test]
fn macho_endianness_and_architectures_map_to_macos() {
    for (magic, cpu, architecture, little) in [
        (
            MACHO_64_LITTLE_MAGIC,
            MACHO_CPU_X86_64,
            RemoteArchitecture::X86_64,
            true,
        ),
        (
            MACHO_64_BIG_MAGIC,
            MACHO_CPU_ARM64,
            RemoteArchitecture::Aarch64,
            false,
        ),
    ] {
        let mut bytes = vec![0; MACHO_HEADER_BYTES];
        bytes[..magic.len()].copy_from_slice(&magic);
        let encoded = if little {
            cpu.to_le_bytes()
        } else {
            cpu.to_be_bytes()
        };
        bytes[MACHO_CPU_OFFSET..MACHO_HEADER_BYTES].copy_from_slice(&encoded);
        let target = binary_target(&bytes).unwrap();
        assert_eq!(target.platform, RemotePlatform::MacOs);
        assert_eq!(target.architecture, architecture);
    }
}

#[test]
fn malformed_or_unsupported_binary_headers_are_rejected() {
    assert!(binary_target(&[]).is_none());
    assert!(binary_target(&[0; ELF_HEADER_BYTES]).is_none());
    let mut bytes = elf(ELF_MACHINE_X86_64, ELF_LITTLE_ENDIAN);
    bytes[ELF_CLASS_OFFSET] = 1;
    assert!(binary_target(&bytes).is_none());
    bytes = elf(ELF_MACHINE_X86_64, ELF_LITTLE_ENDIAN);
    bytes[ELF_VERSION_OFFSET] = 0;
    assert!(binary_target(&bytes).is_none());
    bytes = elf(ELF_MACHINE_X86_64, 0);
    assert!(binary_target(&bytes).is_none());
    bytes = elf(0, ELF_BIG_ENDIAN);
    assert!(binary_target(&bytes).is_none());
    let mut macho = MACHO_64_LITTLE_MAGIC.to_vec();
    macho.resize(MACHO_HEADER_BYTES, 0);
    assert!(binary_target(&macho).is_none());
}

fn elf(machine: u16, endian: u8) -> Vec<u8> {
    let mut bytes = vec![0; ELF_HEADER_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_CLASS_64;
    bytes[ELF_ENDIAN_OFFSET] = endian;
    bytes[ELF_VERSION_OFFSET] = ELF_VERSION_CURRENT;
    let encoded = if endian == ELF_BIG_ENDIAN {
        machine.to_be_bytes()
    } else {
        machine.to_le_bytes()
    };
    bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES].copy_from_slice(&encoded);
    bytes
}
