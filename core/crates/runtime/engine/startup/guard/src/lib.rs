mod codec;
mod constants;
mod frame;
use anyhow::{Result, ensure};
pub use codec::{encode_commit, validate_committed};
pub use constants::{ARGUMENT, ARGUMENT_NAME, CANCELLED, COMMIT_TIMEOUT, MAX_ENCODED_FRAME_BYTES};
use frame::{Frame, Phase};
use std::os::fd::{BorrowedFd, OwnedFd};
use tokio::{
    net::UnixStream,
    time::{Instant, timeout_at},
};

pub struct Sender {
    stream: UnixStream,
}
pub struct Receiver {
    stream: UnixStream,
    deadline: Instant,
}

pub fn channel() -> Result<(Sender, OwnedFd)> {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair()?;
    sender.set_nonblocking(true)?;
    Ok((
        Sender {
            stream: UnixStream::from_std(sender)?,
        },
        receiver.into(),
    ))
}

impl Sender {
    pub async fn commit(mut self, process_id: u32, deadline: Instant) -> Result<()> {
        ensure!(Instant::now() < deadline, CANCELLED);
        Frame::new(process_id, Phase::Commit).validate(process_id, Phase::Commit)?;
        timeout_at(deadline, async {
            frame::write(&mut self.stream, Frame::new(process_id, Phase::Commit)).await?;
            frame::read(&mut self.stream)
                .await?
                .validate(process_id, Phase::Committed)
        })
        .await??;
        ensure!(Instant::now() < deadline, CANCELLED);
        Ok(())
    }
}

impl Receiver {
    pub fn from_stdin() -> Result<Self> {
        let descriptor =
            unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) }.try_clone_to_owned()?;
        let stream = std::os::unix::net::UnixStream::from(descriptor);
        stream.peer_addr()?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream: UnixStream::from_std(stream)?,
            deadline: Instant::now() + COMMIT_TIMEOUT,
        })
    }

    /// EOF, malformed input or timeout cancels an uncommitted service. Once the
    /// commit is accepted, losing its acknowledgement does not undo the commit.
    pub async fn cancellation(mut self) {
        let accepted = timeout_at(self.deadline, async {
            frame::read(&mut self.stream)
                .await?
                .validate(std::process::id(), Phase::Commit)
        })
        .await;
        if !matches!(accepted, Ok(Ok(()))) || Instant::now() >= self.deadline {
            return;
        }
        let _ = timeout_at(
            self.deadline,
            frame::write(
                &mut self.stream,
                Frame::new(std::process::id(), Phase::Committed),
            ),
        )
        .await;
        drop(self);
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
