use super::*;
use crate::paths::process_file_lock::Mode;

const REGISTRATION_UNAVAILABLE: &str =
    "socket namespace recovery requires exclusive durable ownership";
const REGISTRATION_MISSING: &str =
    "socket namespace durable registration is missing; refusing to replace its identity";

/// Lookup never repairs state or creates ownership. Admission is the only writer.
pub(super) fn validate_existing(home: &Path, location: &SocketNamespaceLocation) -> Result<()> {
    let Some(directory) = open_existing(location)? else {
        return Ok(());
    };
    let certificate =
        directory_certificate::read(&directory)?.context(authority::CERTIFICATE_REQUIRED)?;
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    tests::adversarial::after_certificate();
    let identity = registration::inspect(home, &location.binding)?.context(REGISTRATION_MISSING)?;
    anyhow::ensure!(
        certificate.binding == location.binding && certificate.registration == identity,
        authority::CERTIFICATE_MISMATCH
    );
    directory_certificate::verify(&directory, &certificate)
}

pub(super) fn retain(home: &Path, location: SocketNamespaceLocation) -> Result<SocketNamespace> {
    let root = root_selection::open(&location.root, location.overridden_root)?;
    let mut lifecycle = lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root)?;
    let mut existing = open_existing(&location)?;
    if let Some(directory) = existing.as_ref()
        && let Some(certificate) = directory_certificate::read(directory)?
    {
        anyhow::ensure!(
            certificate.binding == location.binding,
            authority::CERTIFICATE_MISMATCH
        );
        let name = location
            .directory
            .file_name()
            .context(authority::CERTIFICATE_REQUIRED)?;
        let certified = root_visit::StageAuthority::capture(directory, name)?
            .context(authority::CERTIFICATE_REQUIRED)?;
        if !certified.cleanup_empty(&root, &mut lifecycle)? {
            let registration = registration::retain(home, &location.binding, false)?;
            return registered(location, root, directory.try_clone()?, registration);
        }
        existing = None;
    }
    // An uncertified directory can be an interrupted initialization. A retained
    // certified directory never enters this branch or recreates lost registration.
    let home_directory = PrivateDirectory::open_or_create_strict(home)?;
    let mut registration = registration::retain(home, &location.binding, true)?;
    home_directory.verify_path()?;
    if let Some(directory) = existing {
        drop(registration);
        let exclusive =
            registration::Registration::try_existing(home, &location.binding, Mode::Exclusive)?
                .context(REGISTRATION_UNAVAILABLE)?;
        exclusive.verify()?;
        unbound::recover(&root, &location, &directory, &mut lifecycle, || {
            exclusive.verify()
        })?;
        exclusive.verify()?;
        drop(exclusive);
        registration = registration::retain(home, &location.binding, false)?;
    }
    let directory = root.ensure_descendant(&location.directory)?;
    directory_certificate::install_or_verify(
        &directory,
        &location.binding,
        registration.durable_identity(),
    )?;
    registered(location, root, directory, registration)
}

fn registered(
    location: SocketNamespaceLocation,
    root: PrivateDirectory,
    directory: PrivateDirectory,
    registration: registration::Registration,
) -> Result<SocketNamespace> {
    let authority = authority::Authority::registered(registration, &directory)?;
    let namespace = SocketNamespace {
        location,
        root,
        directory,
        authority,
        preserve_directory_on_release: false,
    };
    namespace.verify()?;
    Ok(namespace)
}

fn open_existing(location: &SocketNamespaceLocation) -> Result<Option<PrivateDirectory>> {
    match PrivateDirectory::open(&location.directory) {
        Ok(directory) => Ok(Some(directory)),
        Err(error)
            if crate::paths::private_file::io_kind(&error)
                == Some(std::io::ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_admission_tests.rs"]
mod tests;
