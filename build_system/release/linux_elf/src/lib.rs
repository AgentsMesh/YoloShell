use yolo_release_contracts::*;
use yolo_release_foundation::digest::{hex_digest, opened_file_sha256};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

const ELF_OVERFLOW_SENTINEL_BYTES: u64 = 1;
const ELF_CAPTURE_ERROR: &str = "Linux runtime changed while its ELF contract was verified";

pub struct VerifiedLinuxElf {
    pub file: File,
    pub sha256: String,
}

pub fn verify(path: &Path, expected_machine: u16) -> Result<VerifiedLinuxElf> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .with_context(|| format!("open Linux runtime {}", path.display()))?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "Linux runtime must be a regular file");
    ensure!(
        metadata.len() >= ELF64_COMPLETE_HEADER_BYTES as u64
            && metadata.len() <= LINUX_RUNTIME_MAX_BYTES,
        "Linux runtime size is invalid"
    );
    ensure!(
        metadata.permissions().mode() & EXECUTABLE_PERMISSION_MASK != 0,
        "Linux runtime must be executable"
    );
    let expected_size = usize::try_from(metadata.len())?;
    let mut bytes = Vec::with_capacity(expected_size);
    file.by_ref()
        .take(metadata.len() + ELF_OVERFLOW_SENTINEL_BYTES)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() == expected_size, ELF_CAPTURE_ERROR);
    let header = &bytes[..ELF_HEADER_BYTES];
    ensure!(
        &header[..ELF_MAGIC.len()] == ELF_MAGIC,
        "Linux runtime is not ELF"
    );
    ensure!(
        header[ELF_CLASS_OFFSET] == ELF_64_CLASS,
        "Linux runtime must be ELF64"
    );
    ensure!(
        header[ELF_DATA_OFFSET] == ELF_LITTLE_ENDIAN_DATA,
        "Linux runtime must be little-endian"
    );
    let machine = u16::from_le_bytes([
        header[ELF_MACHINE_OFFSET],
        header[ELF_MACHINE_OFFSET + usize::from(ELF_LITTLE_ENDIAN_DATA)],
    ]);
    ensure!(
        machine == expected_machine,
        "Linux runtime architecture mismatch"
    );
    crate::glibc::verify(&bytes)?;
    let parsed_sha256 = hex_digest(Sha256::digest(&bytes));
    let sha256 = opened_file_sha256(&file)?;
    ensure!(parsed_sha256 == sha256, ELF_CAPTURE_ERROR);
    Ok(VerifiedLinuxElf { file, sha256 })
}

#[cfg(test)]
#[path = "../integration_tests/mod.rs"]
mod integration_tests;
mod glibc;
mod reader;
mod versions;
