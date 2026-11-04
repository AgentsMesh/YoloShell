use crate::{BuildImage, constants::IMAGE_CHANGED, evidence, fingerprint::Fingerprint, platform};
use anyhow::{Result, ensure};

/// Keeps exact executable evidence for a running service. Serving the cached
/// description must use image(), which revalidates its mapped/named binding.
pub struct CapturedExecutable {
    opened: platform::Opened,
    fingerprint: Fingerprint,
    image: BuildImage,
}

impl CapturedExecutable {
    pub fn capture() -> Result<Self> {
        Self::from_opened(platform::open_current()?)
    }

    pub fn image(&self) -> Result<&BuildImage> {
        self.verify()?;
        Ok(&self.image)
    }

    pub fn verify(&self) -> Result<()> {
        ensure!(
            Fingerprint::inspect(&self.opened.file.metadata()?)? == self.fingerprint,
            IMAGE_CHANGED
        );
        self.opened.binding.verify(&self.opened.file)?;
        ensure!(
            Fingerprint::inspect(&self.opened.file.metadata()?)? == self.fingerprint,
            IMAGE_CHANGED
        );
        Ok(())
    }

    pub(crate) fn from_opened(mut opened: platform::Opened) -> Result<Self> {
        let (fingerprint, image) =
            evidence::capture(&mut opened.file, |file| opened.binding.verify(file))?;
        let captured = Self {
            opened,
            fingerprint,
            image,
        };
        captured.verify()?;
        Ok(captured)
    }
}

/// One-time observation of this process's running executable, not a disk-version probe.
pub fn capture_current() -> Result<BuildImage> {
    Ok(CapturedExecutable::capture()?.image()?.clone())
}
