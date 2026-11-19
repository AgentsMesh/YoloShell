use super::NamespaceBinding;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Result, ensure};
#[path = "directory_certificate/constants.rs"]
mod constants;
#[path = "directory_certificate/native_xattr.rs"]
mod native_xattr;
#[path = "directory_certificate/record.rs"]
mod record;
use constants::*;
pub(super) use record::Certificate;

pub(super) fn read(directory: &PrivateDirectory) -> Result<Option<Certificate>> {
    let certificate = read_record(directory)?;
    if let Some(certificate) = &certificate {
        certificate.validate(directory.durable_identity()?)?;
    }
    Ok(certificate)
}

/// Verify an admitted immutable record under the caller's retained live fences.
/// This path never recaptures filesystem UUIDs or persistent object handles.
pub(super) fn verify(directory: &PrivateDirectory, expected: &Certificate) -> Result<()> {
    ensure!(
        read_record(directory)?.as_ref() == Some(expected),
        CERTIFICATE_CHANGED
    );
    Ok(())
}

fn read_record(directory: &PrivateDirectory) -> Result<Option<Certificate>> {
    directory.verify_path()?;
    let value = native_xattr::read(directory.descriptor())?;
    directory.verify_path()?;
    let Some(value) = value else {
        return Ok(None);
    };
    let certificate: Certificate = serde_json::from_slice(&value)?;
    Ok(Some(certificate))
}

/// Requires the namespace ownership/lifecycle lease at the call site.
/// Existing certificate bytes are never overwritten, including malformed values.
pub(super) fn install_or_verify(
    directory: &PrivateDirectory,
    expected: &NamespaceBinding,
    registration: crate::paths::identity::DurableObjectIdentity,
) -> Result<()> {
    directory.verify_path()?;
    let certificate = Certificate::new(
        directory.durable_identity()?,
        expected.clone(),
        registration,
    )?;
    let bytes = serde_json::to_vec(&certificate)?;
    native_xattr::create(directory.descriptor(), &bytes)?;
    directory.sync()?;
    ensure!(
        read(directory)?.as_ref() == Some(&certificate),
        CERTIFICATE_CHANGED
    );
    Ok(())
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_directory_certificate_durable_tests.rs"]
mod durable_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../test_support/integration/socket_namespace_directory_certificate_fixture.rs"]
mod fixture;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_directory_certificate_tests.rs"]
mod tests;
