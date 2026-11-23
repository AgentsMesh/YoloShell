use yolo_release_contracts::{
    ELF_64_CLASS, ELF_CLASS_OFFSET, ELF_DATA_OFFSET, ELF_LITTLE_ENDIAN_DATA, ELF_MACHINE_OFFSET,
    ELF_MAGIC, ELF64_COMPLETE_HEADER_BYTES,
};

const ELF_VERSION_OFFSET: usize = 6;
const ELF_CURRENT_VERSION: u8 = 1;
const ELF_MACHINE_BYTES: usize = std::mem::size_of::<u16>();

pub(crate) fn static_elf(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0; ELF64_COMPLETE_HEADER_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_64_CLASS;
    bytes[ELF_DATA_OFFSET] = ELF_LITTLE_ENDIAN_DATA;
    bytes[ELF_VERSION_OFFSET] = ELF_CURRENT_VERSION;
    bytes[ELF_MACHINE_OFFSET..ELF_MACHINE_OFFSET + ELF_MACHINE_BYTES]
        .copy_from_slice(&machine.to_le_bytes());
    bytes
}
