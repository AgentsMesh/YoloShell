use super::*;

pub(super) const CERTIFICATE_REQUIRED: &str = "socket namespace directory certificate is missing";
pub(super) const CERTIFICATE_MISMATCH: &str =
    "socket namespace certificate does not match its durable registration";
const DIRECTORY_LEASE_REQUIRED: &str = "socket namespace participant lease is unavailable";

#[derive(Debug)]
pub(super) struct Authority {
    pub(super) registration: registration::Registration,
    directory: Option<directory_lease::DirectoryLease>,
    certificate: directory_certificate::Certificate,
}

impl Authority {
    pub(super) fn registered(
        registration: registration::Registration,
        directory: &PrivateDirectory,
    ) -> Result<Self> {
        Ok(Self {
            certificate: directory_certificate::read(directory)?.context(CERTIFICATE_REQUIRED)?,
            registration,
            directory: Some(directory_lease::DirectoryLease::shared(directory)?),
        })
    }

    /// The root lifecycle lock excludes concurrent admission while ownership is upgraded.
    pub(super) fn try_upgrade_directory(&mut self, directory: &PrivateDirectory) -> Result<bool> {
        self.directory.take();
        self.directory = directory_lease::DirectoryLease::try_exclusive(directory)?;
        Ok(self.directory.is_some())
    }

    pub(super) fn verify(
        &self,
        directory: &PrivateDirectory,
        expected: &NamespaceBinding,
    ) -> Result<()> {
        self.directory
            .as_ref()
            .context(DIRECTORY_LEASE_REQUIRED)?
            .verify(directory)?;
        self.registration.verify()?;
        anyhow::ensure!(
            self.certificate.binding == *expected
                && self.certificate.registration == self.registration.durable_identity(),
            CERTIFICATE_MISMATCH
        );
        directory_certificate::verify(directory, &self.certificate)
    }
}
