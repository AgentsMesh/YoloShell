use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};
use yolo_paths::{directory::PrivateDirectory, identity::DurableObjectIdentity};

const MARKER: &str = "remount-retained";
const MARKER_BYTES: &[u8] = b"retain this isolated certified namespace across remounts";
const FILE_MODE: u32 = 0o600;
const CERTIFICATE: &std::ffi::CStr = c"user.yolo.namespace";
const CERTIFICATE_BYTES: usize = 2_048;
const CHANGED: &str = "persistent socket namespace or its certificate was replaced";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentNamespace {
    path: PathBuf,
    proof: DurableObjectIdentity,
    certificate: Vec<u8>,
}

impl PersistentNamespace {
    pub fn preserve(directory: PrivateDirectory) -> Result<Self> {
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .open(directory.path().join(MARKER))?;
        marker.write_all(MARKER_BYTES)?;
        marker.sync_all()?;
        directory.sync()?;
        let snapshot = Self {
            path: directory.path().to_owned(),
            proof: directory.durable_identity()?,
            certificate: certificate(&directory)?,
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub fn verify(&self) -> Result<()> {
        let directory = PrivateDirectory::open(&self.path)?;
        ensure!(directory.durable_identity()? == self.proof, CHANGED);
        ensure!(certificate(&directory)? == self.certificate, CHANGED);
        ensure!(
            std::fs::read(self.path.join(MARKER))? == MARKER_BYTES,
            CHANGED
        );
        directory.verify_path()
    }
}

fn certificate(directory: &PrivateDirectory) -> Result<Vec<u8>> {
    let mut bytes = vec![u8::default(); CERTIFICATE_BYTES];
    let length = unsafe {
        libc::fgetxattr(
            directory.descriptor(),
            CERTIFICATE.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if length < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    ensure!(length > 0, CHANGED);
    bytes.truncate(usize::try_from(length)?);
    directory.verify_path()?;
    Ok(bytes)
}
