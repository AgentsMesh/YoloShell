use crate::{constants::*, fingerprint::Fingerprint};
use anyhow::{Result, ensure};
use std::fs::File;

pub(crate) struct Opened {
    pub file: File,
    pub binding: Binding,
}
pub(crate) struct Binding {
    process: u32,
}

pub(crate) fn open_current() -> Result<Opened> {
    let process = std::process::id();
    let file = yolo_process::linux_native::open_executable(process)?;
    Ok(Opened {
        file,
        binding: Binding { process },
    })
}

impl Binding {
    pub fn verify(&self, file: &File) -> Result<()> {
        ensure!(self.process == std::process::id(), WRONG_PROCESS);
        // Reopen the kernel's mapped executable descriptor, never the deploy path.
        let mapped = yolo_process::linux_native::open_executable(self.process)?;
        ensure!(
            Fingerprint::inspect(&file.metadata()?)? == Fingerprint::inspect(&mapped.metadata()?)?,
            IMAGE_CHANGED
        );
        Ok(())
    }
}
