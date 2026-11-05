use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

const REGISTRATION_FILE: &str = ".socket-namespace.json";
const CORRUPT_REGISTRATION: &[u8] = b"{corrupt";

pub(super) struct CorruptRegistration {
    file: File,
    original: Vec<u8>,
}

impl CorruptRegistration {
    pub(super) fn new(home: &Path) -> Self {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(home.join(REGISTRATION_FILE))
            .unwrap();
        let mut original = Vec::new();
        file.read_to_end(&mut original).unwrap();
        file.rewind().unwrap();
        file.set_len(u64::default()).unwrap();
        file.write_all(CORRUPT_REGISTRATION).unwrap();
        file.sync_all().unwrap();
        Self { file, original }
    }
}

impl Drop for CorruptRegistration {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

impl CorruptRegistration {
    fn restore(&mut self) -> std::io::Result<()> {
        self.file.rewind()?;
        self.file.set_len(u64::default())?;
        self.file.write_all(&self.original)?;
        self.file.sync_all()
    }
}
