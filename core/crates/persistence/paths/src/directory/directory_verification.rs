use super::PrivateDirectory;

const PRIVATE_DIRECTORY_PATH_CHANGED_DETAIL: &str = "private directory path changed";

#[derive(Debug)]
pub enum RetainedPathVerification {
    Verified,
    IdentityLost,
    Indeterminate(anyhow::Error),
}

impl PrivateDirectory {
    pub fn verify_path(&self) -> anyhow::Result<()> {
        let reopened = Self::open(&self.path)?;
        anyhow::ensure!(
            reopened.identity == self.identity,
            "{PRIVATE_DIRECTORY_PATH_CHANGED_DETAIL}: {}",
            self.path.display()
        );
        Ok(())
    }

    pub fn verify_retained_path(&self) -> RetainedPathVerification {
        match self.observe_retained_path() {
            RetainedPathVerification::Verified => {}
            settled => return settled,
        }
        match self.verify_path() {
            Ok(()) => RetainedPathVerification::Verified,
            Err(error) => match self.observe_retained_path() {
                RetainedPathVerification::IdentityLost => RetainedPathVerification::IdentityLost,
                RetainedPathVerification::Verified | RetainedPathVerification::Indeterminate(_) => {
                    RetainedPathVerification::Indeterminate(error)
                }
            },
        }
    }

    fn observe_retained_path(&self) -> RetainedPathVerification {
        let observed = std::fs::symlink_metadata(&self.path)
            .map(|metadata| self.identity.matches_metadata(&metadata));
        classify_path_observation(observed)
    }
}

fn classify_path_observation(observed: std::io::Result<bool>) -> RetainedPathVerification {
    match observed {
        Ok(true) => RetainedPathVerification::Verified,
        Ok(false) => RetainedPathVerification::IdentityLost,
        Err(error) if raw_errno_proves_path_identity_loss(error.raw_os_error()) => {
            RetainedPathVerification::IdentityLost
        }
        Err(error) => RetainedPathVerification::Indeterminate(error.into()),
    }
}

pub fn raw_errno_proves_path_identity_loss(errno: Option<libc::c_int>) -> bool {
    matches!(
        errno,
        Some(libc::ENOENT) | Some(libc::ENOTDIR) | Some(libc::ELOOP)
    )
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/directory/directory_verification_inline_tests.rs"]
mod tests;
