use crate::{HubBrokerLaunchScope, constants::*, frame, signature, wire::*};
use anyhow::{Result, ensure};
use std::{
    os::fd::{BorrowedFd, FromRawFd, IntoRawFd},
    sync::Mutex,
};
use yolo_local_unix::BlockingLocalUnixStream;
use yolo_process::ProcessIdentity;
use yolo_remote_broker::{BrokerConfigGeneration, BrokerStatus, IdentityPassphrases};

pub struct HubStartAuthorityGuard {
    scope: HubBrokerLaunchScope,
    has_credentials: bool,
    session: Mutex<Option<Session>>,
}
struct Session {
    stream: BlockingLocalUnixStream,
    child: ChildIdentity,
    nonce: uuid::Uuid,
    sequence: u64,
    committed: Option<BrokerStatus>,
    credentials_sha256: String,
}

impl HubStartAuthorityGuard {
    pub fn from_stdio() -> Result<(Self, IdentityPassphrases)> {
        // Only stdin is admitted. Duplication establishes owned CLOEXEC custody.
        let source = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
        let descriptor = source.try_clone_to_owned()?;
        let stream = std::os::unix::net::UnixStream::from(descriptor);
        stream.peer_addr()?;
        stream.set_nonblocking(false)?;
        let flags = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFD) };
        ensure!(
            flags >= 0
                && unsafe {
                    libc::fcntl(libc::STDIN_FILENO, libc::F_SETFD, flags | libc::FD_CLOEXEC)
                } >= 0,
            INVALID
        );
        Self::receive(unsafe { BlockingLocalUnixStream::from_raw_fd(stream.into_raw_fd()) })
    }
    pub(crate) fn receive(
        mut stream: BlockingLocalUnixStream,
    ) -> Result<(Self, IdentityPassphrases)> {
        let child = ProcessIdentity::capture(std::process::id())?.into();
        let nonce = uuid::Uuid::new_v4();
        let request = Request {
            version: VERSION,
            child,
            nonce,
            sequence: u64::default(),
            action: Action::Hello {},
        };
        let initial = tokio::time::Instant::now() + HANDSHAKE_TIMEOUT;
        frame::write(&mut stream, &request, initial)?;
        let response: SignedStatement = frame::read(&mut stream, initial)?;
        signature::verify(&response, &request)?;
        let scope = response.statement.scope;
        let credentials = frame::read_bytes(&mut stream, scope.deadline()?.min(initial))?;
        ensure!(
            yolo_crypto::sha256_hex(&credentials) == response.statement.credentials_sha256,
            INVALID
        );
        let passphrases = if credentials.is_empty() {
            IdentityPassphrases::none()
        } else {
            IdentityPassphrases::read_checked(credentials.as_slice(), false)
                .map_err(|_| anyhow::anyhow!(INVALID_CREDENTIALS))?
        };
        let session = Session {
            stream,
            child,
            nonce,
            sequence: COUNTER_STEP,
            committed: None,
            credentials_sha256: response.statement.credentials_sha256,
        };
        Ok((
            Self {
                scope,
                has_credentials: !credentials.is_empty(),
                session: Mutex::new(Some(session)),
            },
            passphrases,
        ))
    }
    pub fn scope(&self) -> &HubBrokerLaunchScope {
        &self.scope
    }
    pub fn has_credentials(&self) -> bool {
        self.has_credentials
    }
    pub fn verify(&self) -> Result<()> {
        self.exchange(Action::Verify {}).map(drop)
    }
    pub fn verify_generation(&self, generation: &BrokerConfigGeneration) -> Result<()> {
        self.scope.verify_generation(generation)?;
        self.verify()
    }
    pub fn commit(&self, ready: &BrokerStatus) -> Result<()> {
        self.scope.verify_status(ready)?;
        self.exchange(Action::Commit {
            ready: ready.clone(),
        })
        .map(drop)
    }
    pub fn finished(&self, ready: &BrokerStatus) -> Result<()> {
        self.exchange(Action::Finished {
            ready: ready.clone(),
        })
        .map(drop)
    }
    fn exchange(&self, action: Action) -> Result<()> {
        let mut slot = self.session.lock().map_err(|_| anyhow::anyhow!(POISONED))?;
        let session = slot.as_mut().ok_or_else(|| anyhow::anyhow!(CLOSED))?;
        let result = session.exchange(&self.scope, action);
        if result.is_err() || session.sequence == u64::MAX {
            slot.take();
        }
        result
    }
}

impl Session {
    fn exchange(&mut self, scope: &HubBrokerLaunchScope, action: Action) -> Result<()> {
        scope.verify()?;
        match &action {
            Action::Verify {} | Action::Commit { .. } => ensure!(self.committed.is_none(), CLOSED),
            Action::Finished { ready } => {
                let mut expected = self
                    .committed
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!(CLOSED))?;
                expected.adopted = true;
                ensure!(*ready == expected, SCOPE_MISMATCH);
            }
            Action::Hello {} => anyhow::bail!(INVALID),
        }
        let request = Request {
            version: VERSION,
            child: self.child,
            nonce: self.nonce,
            sequence: self.sequence,
            action: action.clone(),
        };
        let deadline = scope.deadline()?;
        frame::write(&mut self.stream, &request, deadline)?;
        let response: SignedStatement = frame::read(&mut self.stream, deadline)?;
        signature::verify(&response, &request)?;
        ensure!(
            response.statement.scope == *scope
                && response.statement.credentials_sha256 == self.credentials_sha256,
            SCOPE_MISMATCH
        );
        if let Action::Commit { ready } = action {
            self.committed = Some(ready);
        }
        self.sequence = if matches!(request.action, Action::Finished { .. }) {
            u64::MAX
        } else {
            self.sequence
                .checked_add(COUNTER_STEP)
                .ok_or_else(|| anyhow::anyhow!(INVALID))?
        };
        Ok(())
    }
}
