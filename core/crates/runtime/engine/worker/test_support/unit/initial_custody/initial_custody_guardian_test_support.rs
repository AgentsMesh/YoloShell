#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_RUNTIME_ENVIRONMENT: &str = "YOLO_TEST_RUNTIME";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_RUNTIME_DETAIL: &str =
    "initial custody test guardian requires YOLO_TEST_RUNTIME";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_RUNTIME_RESOLUTION_DETAIL: &str =
    "resolve initial custody test guardian runtime";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_SOURCE_ENVIRONMENT: &str = "TEST_SRCDIR";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_WORKSPACE_ENVIRONMENT: &str = "TEST_WORKSPACE";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_HOME_LABEL: &str = "initial custody test home";
#[cfg(target_os = "macos")]
const INITIAL_TEST_GUARDIAN_RUNTIME_MISMATCH_DETAIL: &str =
    "initial custody test guardian runtime changed";

#[cfg(target_os = "macos")]
async fn initial_test_prepared_ack(
    paths: &Paths,
    request: &crate::pty_custody::CustodyRequest,
    offer: &crate::pty_custody::CustodyOffer,
    custody: &crate::process::MacosSecondaryCustody,
) -> Result<crate::pty_custody::CustodyHostAck> {
    initial_test_prepared_ack_with_descriptors(paths, request, offer, &custody.as_descriptors())
        .await
}

#[cfg(not(target_os = "macos"))]
async fn initial_test_prepared_ack(
    _paths: &Paths,
    _request: &crate::pty_custody::CustodyRequest,
    offer: &crate::pty_custody::CustodyOffer,
    _custody: &Vec<std::os::fd::OwnedFd>,
) -> Result<crate::pty_custody::CustodyHostAck> {
    Ok(crate::pty_custody::CustodyHostAck::prepared(offer))
}

#[cfg(target_os = "macos")]
async fn initial_test_prepared_ack_with_descriptors(
    paths: &Paths,
    request: &crate::pty_custody::CustodyRequest,
    offer: &crate::pty_custody::CustodyOffer,
    descriptors: &[std::os::fd::BorrowedFd<'_>],
) -> Result<crate::pty_custody::CustodyHostAck> {
    let namespace = crate::namespace_lease::NamespaceLease::capture(&[(
        INITIAL_TEST_GUARDIAN_HOME_LABEL,
        paths.home(),
    )])?;
    let runtime_path = paths.runtime_record(request.runtime.terminal_id);
    let (runtime, runtime_identity): (RuntimeRecord, _) =
        namespace.read_json_with_identity(&runtime_path)?;
    anyhow::ensure!(
        request.runtime.matches_record(&runtime),
        INITIAL_TEST_GUARDIAN_RUNTIME_MISMATCH_DETAIL
    );
    let custody_path = paths.pty_custody(request.runtime.terminal_id);
    let (durable, durable_identity): (crate::pty_custody::CustodyDurableRecord, _) =
        namespace.read_json_with_identity(&custody_path)?;
    let executable = initial_test_guardian_executable()?;
    let guardian = crate::custody_guardian::spawn_initial(
        &executable,
        paths,
        request.runtime.terminal_id,
        request,
        offer,
        runtime_identity,
        crate::pty_custody::CustodyDurableSnapshot::new(durable, durable_identity),
        descriptors,
    )
    .await?;
    Ok(crate::pty_custody::CustodyHostAck::prepared_guarded(
        offer, guardian,
    ))
}

#[cfg(target_os = "macos")]
fn initial_test_guardian_executable() -> Result<std::path::PathBuf> {
    let configured = std::env::var_os(INITIAL_TEST_GUARDIAN_RUNTIME_ENVIRONMENT)
        .map(std::path::PathBuf::from)
        .context(INITIAL_TEST_GUARDIAN_RUNTIME_DETAIL)?;
    if configured.is_absolute() {
        return Ok(configured);
    }
    if let Ok(resolved) = configured.canonicalize() {
        return Ok(resolved);
    }
    let source = std::env::var_os(INITIAL_TEST_GUARDIAN_SOURCE_ENVIRONMENT)
        .map(std::path::PathBuf::from)
        .context(INITIAL_TEST_GUARDIAN_RUNTIME_RESOLUTION_DETAIL)?;
    let workspace = std::env::var_os(INITIAL_TEST_GUARDIAN_WORKSPACE_ENVIRONMENT)
        .context(INITIAL_TEST_GUARDIAN_RUNTIME_RESOLUTION_DETAIL)?;
    source
        .join(workspace)
        .join(configured)
        .canonicalize()
        .context(INITIAL_TEST_GUARDIAN_RUNTIME_RESOLUTION_DETAIL)
}
