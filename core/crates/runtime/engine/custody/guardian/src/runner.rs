use super::protocol::{GuardianBootstrap, GuardianReady};
use super::socket::GuardianSocket;
use anyhow::{Context, Result, ensure};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::PathBuf;
use uuid::Uuid;

const DUPLICATED_DESCRIPTOR_MINIMUM: libc::c_int = libc::STDERR_FILENO + 1;
const DUPLICATE_FAILURE: libc::c_int = -1;
const BOOTSTRAP_CONTEXT: &str = "adopt detached PTY custody guardian bootstrap socket";
const BOOTSTRAP_DETAIL: &str = "detached PTY custody guardian bootstrap is not exact";
const NAMESPACE_HOME_LABEL: &str = "custody guardian home";
const NAMESPACE_SESSIONS_LABEL: &str = "custody guardian sessions";
const READY_RESPONSE_DETAIL: &str = "detached guardian Ready response was not observed";

pub(super) async fn run(home: PathBuf, terminal_id: Uuid) -> Result<()> {
    let paths = crate::paths::Paths::new(home);
    let _socket_namespace = paths.retain_socket_namespace()?;
    let sessions_dir = paths.sessions_dir();
    let namespace = crate::namespace_lease::NamespaceLease::capture(&[
        (NAMESPACE_HOME_LABEL, paths.home()),
        (NAMESPACE_SESSIONS_LABEL, &sessions_dir),
    ])?;
    let mut bootstrap_stream = inherited_stream()?;
    let bootstrap: GuardianBootstrap = bootstrap_stream.read().await?;
    ensure!(
        bootstrap.request.runtime.terminal_id == terminal_id,
        BOOTSTRAP_DETAIL
    );
    let runtime_path = paths.runtime_record(terminal_id);
    let (runtime, runtime_identity): (crate::model::RuntimeRecord, _) =
        namespace.read_json_with_identity(&runtime_path)?;
    #[cfg(target_os = "macos")]
    let retained_runtime_file = super::settlement_runtime_file::RetainedRuntimeFile::capture(
        &namespace,
        &paths,
        &runtime,
        runtime_identity,
    )?;
    let (durable, durable_identity): (crate::pty_custody::CustodyDurableRecord, _) =
        namespace.read_json_with_identity(&paths.pty_custody(terminal_id))?;
    super::bootstrap_validation::validate(
        &bootstrap,
        &runtime,
        runtime_identity,
        &durable,
        durable_identity,
    )?;
    let descriptors = bootstrap_stream
        .receive_rights(bootstrap.offer.descriptor_set.descriptor_count())
        .await?;
    let custody = import_custody(&bootstrap, descriptors)?;
    let socket_path = super::socket::path_for(&paths, bootstrap.guardian_id)?;
    let socket = GuardianSocket::bind(socket_path)?;
    let process = crate::pty_custody::CustodyHolderIdentity::capture()?;
    let binding = crate::pty_custody::CustodyGuardianBinding::new(
        bootstrap.guardian_id,
        process,
        &bootstrap.request,
        &bootstrap.offer,
    )
    .context(BOOTSTRAP_DETAIL)?;
    #[cfg(target_os = "macos")]
    let preparation = super::preparation::install(
        &namespace,
        &paths,
        &durable,
        durable_identity,
        runtime_identity,
        &bootstrap.request,
        &bootstrap.offer,
        binding,
    )?;
    #[cfg(target_os = "macos")]
    let prepared: (
        crate::pty_custody::CustodyDurableRecord,
        crate::paths::identity::FileIdentity,
    ) = namespace.read_json_with_identity(&paths.pty_custody(terminal_id))?;
    #[cfg(target_os = "macos")]
    ensure!(
        prepared.1 == preparation.custody_file_identity(),
        BOOTSTRAP_DETAIL
    );
    #[cfg(not(target_os = "macos"))]
    let preparation = crate::pty_custody::CustodyGuardianPreparation::new(
        binding,
        bootstrap.runtime_file_identity,
        durable_identity,
        durable_identity,
    );
    if let Err(error) = bootstrap_stream
        .write(&GuardianReady::new(preparation))
        .await
    {
        super::terminal_reporting::warn_owned(error, READY_RESPONSE_DETAIL);
    }
    drop(bootstrap_stream);
    #[cfg(target_os = "macos")]
    return super::settlement::run(
        namespace,
        paths,
        socket,
        custody,
        preparation,
        prepared.0,
        crate::pty_custody::CustodyDurableSnapshot::new(durable, durable_identity),
        runtime,
        runtime_identity,
        retained_runtime_file,
    )
    .await;
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!(BOOTSTRAP_DETAIL)
}

fn inherited_stream() -> Result<crate::pty_custody::CustodyUnixStream> {
    let descriptor = unsafe {
        libc::fcntl(
            libc::STDIN_FILENO,
            libc::F_DUPFD_CLOEXEC,
            DUPLICATED_DESCRIPTOR_MINIMUM,
        )
    };
    ensure!(descriptor != DUPLICATE_FAILURE, BOOTSTRAP_CONTEXT);
    let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
    let stream = std::os::unix::net::UnixStream::from(descriptor);
    stream.set_nonblocking(true)?;
    let stream = crate::local_unix::LocalUnixStream::from_std(stream)?;
    crate::pty_custody::CustodyUnixStream::from_local(stream).context(BOOTSTRAP_CONTEXT)
}

#[cfg(target_os = "macos")]
fn import_custody(
    bootstrap: &GuardianBootstrap,
    descriptors: Vec<OwnedFd>,
) -> Result<crate::process::MacosSecondaryCustody> {
    let ordered: [OwnedFd; crate::process::MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] = descriptors
        .try_into()
        .map_err(|_| anyhow::anyhow!(BOOTSTRAP_DETAIL))?;
    crate::process::MacosSecondaryCustody::from_transferred(
        crate::process::MacosSecondaryCustodyDescriptorParts::from_ordered(ordered),
        &bootstrap.request.runtime,
        bootstrap.offer.recovery_identity_sha256,
        bootstrap.offer.descriptor_set,
        bootstrap.offer.profile,
    )
}

#[cfg(not(target_os = "macos"))]
fn import_custody(_bootstrap: &GuardianBootstrap, _descriptors: Vec<OwnedFd>) -> Result<()> {
    anyhow::bail!(BOOTSTRAP_DETAIL)
}
