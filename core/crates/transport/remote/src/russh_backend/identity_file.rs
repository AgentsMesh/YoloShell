use super::super::RemoteError;
use super::super::constants::{
    SSH_IDENTITY_DECODE_FAILURE_CONTEXT, SSH_IDENTITY_UTF8_FAILURE_CONTEXT,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct IdentityFile {
    path: PathBuf,
    material: Option<Arc<[u8]>>,
    passphrase: Option<Arc<Zeroizing<String>>>,
}

impl IdentityFile {
    pub fn new(path: impl Into<PathBuf>, passphrase: Option<String>) -> Self {
        Self {
            path: path.into(),
            material: None,
            passphrase: passphrase.map(Zeroizing::new).map(Arc::new),
        }
    }

    pub fn from_material(
        path: impl Into<PathBuf>,
        material: Arc<[u8]>,
        passphrase: Option<Arc<Zeroizing<String>>>,
    ) -> Result<Self, RemoteError> {
        let secret = material_text(&material)?;
        russh::keys::decode_secret_key(secret, passphrase.as_deref().map(|value| value.as_str()))
            .map_err(|error| {
            RemoteError::transport(format!("{SSH_IDENTITY_DECODE_FAILURE_CONTEXT}: {error}"))
        })?;
        Ok(Self {
            path: path.into(),
            material: Some(material),
            passphrase,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn material(&self) -> Option<&[u8]> {
        self.material.as_deref()
    }

    #[cfg(feature = "broker-test-support")]
    #[doc(hidden)]
    pub fn material_for_broker_test(&self) -> Option<&[u8]> {
        self.material()
    }

    pub(super) fn passphrase(&self) -> Option<&str> {
        self.passphrase.as_deref().map(|value| value.as_str())
    }
}

pub(super) fn material_text(material: &[u8]) -> Result<&str, RemoteError> {
    std::str::from_utf8(material).map_err(|error| {
        RemoteError::transport(format!("{SSH_IDENTITY_UTF8_FAILURE_CONTEXT}: {error}"))
    })
}
