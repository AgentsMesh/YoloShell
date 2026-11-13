use super::constants::*;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
};

pub fn image(path: &Path, bytes: &[u8]) -> File {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE))
        .unwrap();
    file
}

pub fn digest(mut file: File) -> String {
    use sha2::{Digest, Sha256};
    use std::{
        fmt::Write as _,
        io::{Read, Seek},
    };
    file.rewind().unwrap();
    let mut hash = Sha256::new();
    let mut bytes = [u8::default(); TEST_BUFFER_BYTES];
    loop {
        let count = file.read(&mut bytes).unwrap();
        if count == usize::default() {
            break;
        }
        hash.update(&bytes[..count]);
    }
    let mut encoded = String::new();
    for byte in hash.finalize() {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}
