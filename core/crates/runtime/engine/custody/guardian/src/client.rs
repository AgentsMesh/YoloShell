use super::protocol::{GUARDIAN_PREPARE_METHOD, GuardianPrepareRequest, GuardianPrepared};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableSnapshot;
use crate::pty_custody::{CustodyGuardianBinding, CustodyOffer, CustodyRequest, CustodyUnixStream};
use anyhow::{Context, Result, ensure};

const CONNECT_CONTEXT: &str = "connect to detached PTY custody guardian";
const RESPONSE_DETAIL: &str = "detached PTY custody guardian returned another successor binding";
const PREPARE_ATTEMPTS: usize = 2;

pub async fn prepare_existing(
    paths: &crate::paths::Paths,
    current: CustodyGuardianBinding,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: &CustodyDurableSnapshot,
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    ensure!(current.process().matches(), CONNECT_CONTEXT);
    let expected =
        CustodyGuardianBinding::new(current.guardian_id(), current.process(), request, offer)
            .context(RESPONSE_DETAIL)?;
    let mut failure = None;
    for _ in 0..PREPARE_ATTEMPTS {
        match prepare_once(
            paths,
            current,
            request,
            offer,
            runtime_file_identity,
            durable_predecessor,
            expected,
        )
        .await
        {
            Ok(preparation) => return Ok(preparation),
            Err(error) => failure = Some(error),
        }
    }
    Err(failure.unwrap_or_else(|| anyhow::anyhow!(RESPONSE_DETAIL)))
}

async fn prepare_once(
    paths: &crate::paths::Paths,
    current: CustodyGuardianBinding,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: &CustodyDurableSnapshot,
    expected: CustodyGuardianBinding,
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    let path = super::socket::path_for(paths, current.guardian_id())?;
    super::socket::verify_named(&path)?;
    let mut stream = CustodyUnixStream::connect(&path)
        .await
        .context(CONNECT_CONTEXT)?;
    super::socket::verify_peer(&stream, current.process())?;
    let payload = GuardianPrepareRequest::new(
        current.guardian_id(),
        request.clone(),
        offer.clone(),
        runtime_file_identity,
        durable_predecessor.clone(),
    );
    stream
        .write_method_opening(GUARDIAN_PREPARE_METHOD, &payload)
        .await?;
    let prepared: GuardianPrepared = stream.read().await?;
    ensure!(prepared.is_current(), RESPONSE_DETAIL);
    ensure!(prepared.validates(expected), RESPONSE_DETAIL);
    Ok(prepared.preparation)
}
