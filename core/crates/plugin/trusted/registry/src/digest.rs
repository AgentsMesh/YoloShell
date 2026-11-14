use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub fn source_digest(root: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let mut hash = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let relative = relative.to_string_lossy();
        let bytes = std::fs::read(&path)?;
        hash.update((relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(hex(&hash.finalize()))
}

fn collect(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "plugin source must not contain symlinks",
            ));
        }
        if metadata.is_dir() {
            collect(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "plugin source contains a non-file entry",
            ));
        }
    }
    if root == directory && files.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "plugin source is empty",
        ));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    const HEX_CHARS_PER_BYTE: usize = 2;
    let mut output = String::with_capacity(bytes.len() * HEX_CHARS_PER_BYTE);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}
