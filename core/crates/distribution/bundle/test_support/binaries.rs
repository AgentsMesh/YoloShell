use yolo_distribution_bundle::{Architecture, Platform, Target};

pub const ELF_MAGIC: &[u8] = b"\x7fELF";
pub const ELF_BYTES: usize = 20;
pub const ELF_CLASS_OFFSET: usize = 4;
pub const ELF_ENDIAN_OFFSET: usize = 5;
pub const ELF_VERSION_OFFSET: usize = 6;
pub const ELF_MACHINE_OFFSET: usize = 18;
pub const ELF_64: u8 = 2;
pub const ELF_CURRENT: u8 = 1;
pub const LITTLE_ENDIAN: u8 = 1;
pub const BIG_ENDIAN: u8 = 2;
pub const ELF_X86_64: u16 = 62;
pub const ELF_AARCH64: u16 = 183;
pub const MACHO_LITTLE: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];
pub const MACHO_BIG: [u8; 4] = [0xfe, 0xed, 0xfa, 0xcf];
pub const MACHO_BYTES: usize = 8;
pub const MACHO_CPU_OFFSET: usize = 4;
pub const MACHO_X86_64: u32 = 0x0100_0007;
pub const MACHO_AARCH64: u32 = 0x0100_000c;
pub const INVALID_HEADER: u8 = 0;

pub fn all_targets() -> [Target; 4] {
    [
        Target {
            platform: Platform::Linux,
            architecture: Architecture::Aarch64,
        },
        Target {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        },
        Target {
            platform: Platform::MacOs,
            architecture: Architecture::Aarch64,
        },
        Target {
            platform: Platform::MacOs,
            architecture: Architecture::X86_64,
        },
    ]
}

pub fn binary(target: Target, little: bool) -> Vec<u8> {
    match target.platform {
        Platform::Linux => elf(target.architecture, little),
        Platform::MacOs => macho(target.architecture, little),
    }
}

fn elf(architecture: Architecture, little: bool) -> Vec<u8> {
    let machine = match architecture {
        Architecture::Aarch64 => ELF_AARCH64,
        Architecture::X86_64 => ELF_X86_64,
    };
    let mut bytes = vec![0; ELF_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_64;
    bytes[ELF_VERSION_OFFSET] = ELF_CURRENT;
    bytes[ELF_ENDIAN_OFFSET] = if little { LITTLE_ENDIAN } else { BIG_ENDIAN };
    let encoded = if little {
        machine.to_le_bytes()
    } else {
        machine.to_be_bytes()
    };
    bytes[ELF_MACHINE_OFFSET..].copy_from_slice(&encoded);
    bytes
}

fn macho(architecture: Architecture, little: bool) -> Vec<u8> {
    let cpu = match architecture {
        Architecture::Aarch64 => MACHO_AARCH64,
        Architecture::X86_64 => MACHO_X86_64,
    };
    let mut bytes = vec![0; MACHO_BYTES];
    bytes[..MACHO_CPU_OFFSET].copy_from_slice(if little { &MACHO_LITTLE } else { &MACHO_BIG });
    let encoded = if little {
        cpu.to_le_bytes()
    } else {
        cpu.to_be_bytes()
    };
    bytes[MACHO_CPU_OFFSET..].copy_from_slice(&encoded);
    bytes
}
