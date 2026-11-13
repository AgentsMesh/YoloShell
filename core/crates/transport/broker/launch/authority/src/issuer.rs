use crate::{HubBrokerLaunchScope, constants::*, frame, signature, wire::*};
use anyhow::{Result, ensure};
use std::{os::fd::OwnedFd, sync::Arc};
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;
use yolo_process::{ProcessIdentity, ScopedWaitableChild};
use yolo_remote_broker::BrokerStatus;
use zeroize::Zeroizing;

pub struct LaunchIssuer {
    owner: Arc<HubState>,
    scope: HubBrokerLaunchScope,
    stream: LocalUnixStream,
    child: Option<ProcessIdentity>,
    credentials: Option<Zeroizing<Vec<u8>>>,
    verify_current: Box<dyn Fn() -> Result<()> + Send + Sync>,
}

pub fn issue(
    owner: Arc<HubState>,
    scope: HubBrokerLaunchScope,
    credentials: Option<Zeroizing<Vec<u8>>>,
    verify_current: impl Fn() -> Result<()> + Send + Sync + 'static,
) -> Result<(LaunchIssuer, OwnedFd)> {
    scope.verify()?;
    owner.verify()?;
    ensure!(
        scope.owner == owner.peer_identity() && scope.hub_state == owner.directory()?.path(),
        SCOPE_MISMATCH
    );
    verify_current()?;
    if let Some(bytes) = &credentials {
        yolo_remote_broker::IdentityPassphrases::read_checked(bytes.as_slice(), false)
            .map_err(|_| anyhow::anyhow!(INVALID_CREDENTIALS))?;
    }
    let (stream, inherited) = std::os::unix::net::UnixStream::pair()?;
    stream.set_nonblocking(true)?;
    Ok((
        LaunchIssuer {
            owner,
            scope,
            stream: LocalUnixStream::from_std(stream)?,
            child: None,
            credentials,
            verify_current: Box::new(verify_current),
        },
        inherited.into(),
    ))
}

impl LaunchIssuer {
    /// Caller proves custody with the original captured child; a claimed PID is insufficient.
    pub fn bind_child(&mut self, child: &ScopedWaitableChild) -> Result<()> {
        ensure!(self.child.is_none(), CHILD_MISMATCH);
        let identity = child
            .captured_identity()
            .ok_or_else(|| anyhow::anyhow!(CHILD_MISMATCH))?;
        ensure!(identity.matches(), CHILD_MISMATCH);
        self.verify()?;
        self.child = Some(identity);
        Ok(())
    }

    /// Returns only after the bound child reports its actual supervisor handoff completed.
    pub async fn serve(mut self) -> Result<BrokerStatus> {
        let child = self.child.ok_or_else(|| anyhow::anyhow!(CHILD_MISMATCH))?;
        let deadline = self.scope.deadline()?;
        let hello: Request = frame::read_async(&mut self.stream, deadline).await?;
        hello.validate()?;
        ensure!(
            hello.child == child.into()
                && hello.sequence == u64::default()
                && matches!(hello.action, Action::Hello {}),
            CHILD_MISMATCH
        );
        self.verify_child(child)?;
        let credentials = self.credentials.take().unwrap_or_default();
        let digest = yolo_crypto::sha256_hex(&credentials);
        self.reply(hello.clone(), digest.clone(), deadline).await?;
        frame::write_bytes_async(&mut self.stream, &credentials, deadline).await?;
        drop(credentials);
        let mut sequence = COUNTER_STEP;
        let mut committed: Option<BrokerStatus> = None;
        loop {
            let request: Request = frame::read_async(&mut self.stream, deadline).await?;
            request.validate()?;
            ensure!(
                request.child == child.into()
                    && request.nonce == hello.nonce
                    && request.sequence == sequence,
                INVALID
            );
            self.verify_child(child)?;
            let finished = match &request.action {
                Action::Verify {} => {
                    ensure!(committed.is_none(), CLOSED);
                    None
                }
                Action::Commit { ready } => {
                    ensure!(committed.is_none(), CLOSED);
                    self.scope.verify_status(ready)?;
                    committed = Some(ready.clone());
                    None
                }
                Action::Finished { ready } => {
                    self.scope.verify_status(ready)?;
                    let mut expected = committed.take().ok_or_else(|| anyhow::anyhow!(CLOSED))?;
                    expected.adopted = true;
                    ensure!(*ready == expected, SCOPE_MISMATCH);
                    Some(ready.clone())
                }
                Action::Hello {} => anyhow::bail!(INVALID),
            };
            self.reply(request, digest.clone(), deadline).await?;
            if let Some(ready) = finished {
                return Ok(ready);
            }
            sequence = sequence
                .checked_add(COUNTER_STEP)
                .ok_or_else(|| anyhow::anyhow!(INVALID))?;
        }
    }
    fn verify_child(&self, child: ProcessIdentity) -> Result<()> {
        self.verify()?;
        ensure!(child.matches(), CHILD_MISMATCH);
        Ok(())
    }
    fn verify(&self) -> Result<()> {
        self.owner.verify()?;
        self.scope.verify()?;
        (self.verify_current)()
    }
    async fn reply(
        &mut self,
        request: Request,
        credentials_sha256: String,
        deadline: tokio::time::Instant,
    ) -> Result<()> {
        self.verify()?;
        let response = signature::sign(
            &self.owner,
            Statement {
                scope: self.scope.clone(),
                request,
                credentials_sha256,
            },
        )?;
        frame::write_async(&mut self.stream, &response, deadline).await
    }
}
