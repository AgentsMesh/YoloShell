use super::protocol::{GuardianBootstrap, GuardianReady};
use super::spawn_settlement::GuardianSpawnSettlement;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableSnapshot, CustodyGuardianBinding, CustodyOffer, CustodyRequest, CustodyUnixStream,
};
use anyhow::{Context, Result, ensure};
use std::future::Future;
use std::os::fd::{BorrowedFd, OwnedFd};
use std::path::Path;
use std::process::Stdio;
use uuid::Uuid;

const SPAWN_CONTEXT: &str = "spawn detached PTY custody guardian";
const SPAWN_NAMESPACE_CONTEXT: &str = "retain detached guardian launch namespace";
const READY_DETAIL: &str = "detached PTY custody guardian returned another binding";
const READY_RECOVERY_CONTEXT: &str = "recover detached guardian after Ready response loss";

pub async fn spawn_initial(
    executable: &Path,
    paths: &Paths,
    terminal_id: Uuid,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: CustodyDurableSnapshot,
    descriptors: &[BorrowedFd<'_>],
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    spawn(
        executable,
        paths,
        terminal_id,
        request,
        offer,
        runtime_file_identity,
        durable_predecessor,
        descriptors,
    )
    .await
}

pub async fn spawn_successor(
    executable: &Path,
    paths: &Paths,
    terminal_id: Uuid,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: CustodyDurableSnapshot,
    descriptors: &[BorrowedFd<'_>],
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    spawn(
        executable,
        paths,
        terminal_id,
        request,
        offer,
        runtime_file_identity,
        durable_predecessor,
        descriptors,
    )
    .await
}

async fn spawn(
    executable: &Path,
    paths: &Paths,
    terminal_id: Uuid,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: CustodyDurableSnapshot,
    descriptors: &[BorrowedFd<'_>],
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    let namespace = paths
        .retain_socket_namespace()
        .context(SPAWN_NAMESPACE_CONTEXT)?;
    let guardian_id = Uuid::new_v4();
    let (parent, child) = std::os::unix::net::UnixStream::pair()?;
    parent.set_nonblocking(true)?;
    let child_input: OwnedFd = child.into();
    let mut command = super::spawn_command::command(
        executable,
        paths,
        terminal_id,
        &namespace,
        Stdio::from(child_input),
    );
    let mut settlement = GuardianSpawnSettlement::spawn(&mut command).context(SPAWN_CONTEXT)?;
    let preparation = complete_spawn(
        &mut settlement,
        parent,
        paths,
        guardian_id,
        request,
        offer,
        runtime_file_identity,
        durable_predecessor,
        descriptors,
    )
    .await?;
    namespace.verify().context(SPAWN_NAMESPACE_CONTEXT)?;
    settlement.retain_naturally()?;
    Ok(preparation)
}

async fn complete_spawn(
    settlement: &mut GuardianSpawnSettlement,
    parent: std::os::unix::net::UnixStream,
    paths: &Paths,
    guardian_id: Uuid,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: CustodyDurableSnapshot,
    descriptors: &[BorrowedFd<'_>],
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    let child = settlement.child()?;
    let process = crate::pty_custody::CustodyHolderIdentity {
        pid: child.id(),
        start_time: crate::process::process_start_time(child.id())?,
    };
    let expected =
        CustodyGuardianBinding::new(guardian_id, process, request, offer).context(READY_DETAIL)?;
    #[cfg(any(test, feature = "host-test-hooks"))]
    super::test_processes::register(paths.home(), request.runtime.terminal_id, expected.process())?;
    let bootstrap = GuardianBootstrap::new(
        guardian_id,
        request.clone(),
        offer.clone(),
        runtime_file_identity,
        durable_predecessor.clone(),
    );
    match exchange(settlement, parent, bootstrap, descriptors, expected).await {
        Ok(preparation) => Ok(preparation),
        Err(original) => super::client::prepare_existing(
            paths,
            expected,
            request,
            offer,
            runtime_file_identity,
            &durable_predecessor,
        )
        .await
        .map_err(|recovery| original.context(format!("{READY_RECOVERY_CONTEXT}: {recovery:#}"))),
    }
}

async fn exchange(
    settlement: &mut GuardianSpawnSettlement,
    parent: std::os::unix::net::UnixStream,
    bootstrap: GuardianBootstrap,
    descriptors: &[BorrowedFd<'_>],
    expected: CustodyGuardianBinding,
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    let stream = crate::local_unix::LocalUnixStream::from_std(parent)?;
    let mut stream = CustodyUnixStream::from_local(stream)?;
    let exchange = async {
        stream.write(&bootstrap).await?;
        after_publication_boundary(settlement, stream.send_rights(descriptors)).await?;
        let ready: GuardianReady = stream.read().await?;
        ensure!(ready.validates(expected), READY_DETAIL);
        Ok(ready.preparation)
    };
    tokio::time::timeout(crate::session_timing::WORKER_READY_TIMEOUT, exchange)
        .await
        .context(READY_DETAIL)?
}

async fn after_publication_boundary<Output>(
    settlement: &mut GuardianSpawnSettlement,
    transmission: impl Future<Output = Output>,
) -> Output {
    settlement.mark_publication_possible();
    transmission.await
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/spawn/spawn_tests.rs"]
mod tests;
