use super::*;

#[derive(Debug)]
pub enum RetainedPrivateFileVerification {
    Verified,
    IdentityLost,
    Indeterminate(anyhow::Error),
}

impl PrivateFileLock {
    pub fn verify_retained_path(&self) -> RetainedPrivateFileVerification {
        let descriptor =
            match FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_bytes)
            {
                Ok(identity) => identity,
                Err(error) => return self.classify_named_after_descriptor_error(error),
            };
        if !descriptor.unchanged(self.identity) {
            return RetainedPrivateFileVerification::IdentityLost;
        }
        match self.entry.inspect(self.max_bytes) {
            Ok(Some(named)) if named.unchanged(self.identity) => {
                RetainedPrivateFileVerification::Verified
            }
            Ok(_) => RetainedPrivateFileVerification::IdentityLost,
            Err(error) if error_proves_identity_loss(&error) => {
                RetainedPrivateFileVerification::IdentityLost
            }
            Err(error) => RetainedPrivateFileVerification::Indeterminate(error),
        }
    }

    fn classify_named_after_descriptor_error(
        &self,
        descriptor_error: anyhow::Error,
    ) -> RetainedPrivateFileVerification {
        match self.entry.inspect(self.max_bytes) {
            Ok(Some(named)) if named.unchanged(self.identity) => {
                RetainedPrivateFileVerification::Indeterminate(descriptor_error)
            }
            Ok(_) => RetainedPrivateFileVerification::IdentityLost,
            Err(error) if error_proves_identity_loss(&error) => {
                RetainedPrivateFileVerification::IdentityLost
            }
            Err(_) => RetainedPrivateFileVerification::Indeterminate(descriptor_error),
        }
    }
}

fn error_proves_identity_loss(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| {
                crate::paths::directory::raw_errno_proves_path_identity_loss(error.raw_os_error())
            })
    })
}
