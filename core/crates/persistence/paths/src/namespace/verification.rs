use super::*;
use crate::paths::directory::RetainedPathVerification;

#[derive(Debug)]
pub enum NamespaceVerification {
    Verified,
    IdentityLost {
        proof: NamespaceOwnershipLoss,
        error: anyhow::Error,
    },
    Indeterminate(anyhow::Error),
}

impl NamespaceVerification {
    pub fn into_result(self) -> Result<()> {
        match self {
            Self::Verified => Ok(()),
            Self::IdentityLost { error, .. } | Self::Indeterminate(error) => Err(error),
        }
    }

    pub fn into_ownership_loss(self) -> Option<NamespaceOwnershipLoss> {
        match self {
            Self::IdentityLost { proof, .. } => Some(proof),
            Self::Verified | Self::Indeterminate(_) => None,
        }
    }
}

impl NamespaceLease {
    pub fn verify(&self) -> Result<()> {
        self.verification().into_result()
    }

    pub fn verification(&self) -> NamespaceVerification {
        self.verification_with(|anchor| anchor.directory.verify_retained_path())
    }

    pub fn prove_ownership_loss(&self) -> Option<NamespaceOwnershipLoss> {
        self.verification().into_ownership_loss()
    }

    fn verification_with<F>(&self, mut verify_anchor: F) -> NamespaceVerification
    where
        F: FnMut(&NamespaceAnchor) -> RetainedPathVerification,
    {
        if self.lost.load(Ordering::Acquire) {
            return identity_lost(anyhow::anyhow!(ALREADY_LOST_ERROR));
        }
        for anchor in &self.anchors {
            match verify_anchor(anchor) {
                RetainedPathVerification::Verified => {}
                RetainedPathVerification::IdentityLost => {
                    self.lost.store(true, Ordering::Release);
                    let error = anyhow::anyhow!("{CHANGED_ERROR}: {}", anchor.label);
                    return identity_lost(error);
                }
                RetainedPathVerification::Indeterminate(error) => {
                    return NamespaceVerification::Indeterminate(
                        error.context(format!("{INDETERMINATE_ERROR}: {}", anchor.label)),
                    );
                }
            }
        }
        NamespaceVerification::Verified
    }
}

fn identity_lost(error: anyhow::Error) -> NamespaceVerification {
    NamespaceVerification::IdentityLost {
        proof: NamespaceOwnershipLoss { _private: () },
        error,
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/namespace/verification_inline_tests.rs"]
mod tests;
