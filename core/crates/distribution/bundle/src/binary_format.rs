use crate::{Architecture, Platform, Target, constants::*};

/// Detects supported binary headers; executable validity is checked by release probes.
pub fn binary_target(bytes: &[u8]) -> Option<Target> {
    elf_target(bytes).or_else(|| macho_target(bytes))
}

fn elf_target(bytes: &[u8]) -> Option<Target> {
    if bytes.len() < ELF_HEADER_BYTES
        || !bytes.starts_with(ELF_MAGIC)
        || bytes[ELF_CLASS_OFFSET] != ELF_CLASS_64
        || bytes[ELF_VERSION_OFFSET] != ELF_VERSION_CURRENT
    {
        return None;
    }
    let encoded = bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES]
        .try_into()
        .ok()?;
    let machine = match bytes[ELF_ENDIAN_OFFSET] {
        ELF_LITTLE_ENDIAN => u16::from_le_bytes(encoded),
        ELF_BIG_ENDIAN => u16::from_be_bytes(encoded),
        _ => return None,
    };
    let architecture = match machine {
        ELF_MACHINE_AARCH64 => Architecture::Aarch64,
        ELF_MACHINE_X86_64 => Architecture::X86_64,
        _ => return None,
    };
    Some(Target {
        platform: Platform::Linux,
        architecture,
    })
}

fn macho_target(bytes: &[u8]) -> Option<Target> {
    if bytes.len() < MACHO_HEADER_BYTES {
        return None;
    }
    let encoded = bytes[MACHO_CPU_OFFSET..MACHO_HEADER_BYTES]
        .try_into()
        .ok()?;
    let cpu = if bytes.starts_with(&MACHO_64_LITTLE_MAGIC) {
        u32::from_le_bytes(encoded)
    } else if bytes.starts_with(&MACHO_64_BIG_MAGIC) {
        u32::from_be_bytes(encoded)
    } else {
        return None;
    };
    let architecture = match cpu {
        MACHO_CPU_ARM64 => Architecture::Aarch64,
        MACHO_CPU_X86_64 => Architecture::X86_64,
        _ => return None,
    };
    Some(Target {
        platform: Platform::MacOs,
        architecture,
    })
}
