use crate::supervision_frame::{self as frame, Control};
use anyhow::{Context, Result, ensure};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::process::Command;
use yolo_local_unix::BlockingLocalUnixStream;

const ACTIVATION_MARKER_INVALID: &str = "broker supervision activation marker is invalid";
const ACTIVATION_FD_INVALID: &str = "broker supervision activation descriptor is invalid";
const ACTIVATION_FD_FLAGS_CONTEXT: &str = "secure broker supervision activation descriptor";
const ACTIVATION_READ_CONTEXT: &str = "read broker supervision activation";
const ACTIVATION_WRITE_CONTEXT: &str = "publish broker supervision activation";
const ADOPTION_MARKER_INVALID: &str = "broker supervision adoption marker is invalid";
const ADOPTION_READ_CONTEXT: &str = "read broker supervision adoption";
const ADOPTION_WRITE_CONTEXT: &str = "publish broker supervision adoption";
const LIFETIME_THREAD_CONTEXT: &str = "start broker supervisor lifetime monitor";
const LIFETIME_THREAD_NAME: &str = "yolo-broker-supervisor-lifetime";
const READINESS_CONTEXT: &str = "exchange broker supervision readiness";

pub(super) struct ActivationSender(BlockingLocalUnixStream);
pub(super) struct ActivationReceiver(BlockingLocalUnixStream);
pub(super) struct ReadinessPublisher(BlockingLocalUnixStream);

pub(super) struct SupervisionSignals {
    pub(super) adoption:
        tokio::sync::oneshot::Receiver<yolo_remote_broker::BrokerAdoptionAcknowledgement>,
    pub(super) lost: tokio::sync::oneshot::Receiver<()>,
}

pub(super) use yolo_broker_supervision_state::BrokerReadiness;

pub(super) fn channel() -> Result<(ActivationSender, ActivationReceiver)> {
    let (sender, receiver) = BlockingLocalUnixStream::pair()?;
    Ok((ActivationSender(sender), ActivationReceiver(receiver)))
}

impl ActivationSender {
    pub(super) fn activate(&mut self) -> Result<()> {
        frame::write(&mut self.0, &Control::Activate).context(ACTIVATION_WRITE_CONTEXT)
    }

    pub(super) fn prepare_adoption(
        &mut self,
        instance_id: uuid::Uuid,
        deadline: std::time::Instant,
    ) -> Result<super::supervision_ack::PreparedAdoption> {
        let nonce = uuid::Uuid::new_v4();
        super::supervision_ack::set_deadline(&self.0, deadline)?;
        frame::write(&mut self.0, &Control::Adopt { nonce }).context(ADOPTION_WRITE_CONTEXT)?;
        super::supervision_ack::read_prepared(&mut self.0, nonce, instance_id, deadline)
    }

    pub(super) fn commit_adoption(
        &mut self,
        prepared: super::supervision_ack::PreparedAdoption,
        deadline: std::time::Instant,
    ) -> Result<()> {
        super::supervision_ack::commit(&mut self.0, prepared, deadline)
    }

    pub(super) fn read_readiness(
        &mut self,
        deadline: std::time::Instant,
    ) -> Result<BrokerReadiness> {
        super::supervision_ack::set_deadline(&self.0, deadline)?;
        frame::read(&mut self.0).context(READINESS_CONTEXT)
    }
}

impl ActivationReceiver {
    pub(super) fn descriptor(&self) -> RawFd {
        self.0.as_raw_fd()
    }

    pub(super) fn configure_inheritance(&self, command: &mut Command) {
        let descriptor = self.descriptor();
        unsafe {
            use std::os::unix::process::CommandExt;
            command.pre_exec(move || {
                if libc::fcntl(descriptor, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    pub(super) fn inherit(descriptor: RawFd) -> Result<Self> {
        ensure!(descriptor > libc::STDERR_FILENO, ACTIVATION_FD_INVALID);
        let stream = unsafe { BlockingLocalUnixStream::from_raw_fd(descriptor) };
        if unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return Err(std::io::Error::last_os_error()).context(ACTIVATION_FD_FLAGS_CONTEXT);
        }
        Ok(Self(stream))
    }

    pub(super) fn await_activation(mut self) -> Result<Self> {
        let control: Control = frame::read(&mut self.0).context(ACTIVATION_READ_CONTEXT)?;
        ensure!(
            matches!(control, Control::Activate),
            ACTIVATION_MARKER_INVALID
        );
        Ok(self)
    }
    pub(super) fn readiness_publisher(&self) -> Result<ReadinessPublisher> {
        self.0
            .try_clone()
            .map(ReadinessPublisher)
            .context(READINESS_CONTEXT)
    }

    pub(super) fn monitor_supervisor(
        self,
        deadline: std::time::Instant,
    ) -> Result<SupervisionSignals> {
        let (adoption_send, adoption) = tokio::sync::oneshot::channel();
        let (lost_send, lost) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name(LIFETIME_THREAD_NAME.to_owned())
            .spawn(move || monitor_lifetime(self.0, adoption_send, lost_send, deadline))
            .context(LIFETIME_THREAD_CONTEXT)?;
        Ok(SupervisionSignals { adoption, lost })
    }
}

impl ReadinessPublisher {
    pub(super) fn publish(mut self, readiness: &BrokerReadiness) -> Result<()> {
        frame::write(&mut self.0, readiness).context(READINESS_CONTEXT)
    }
}

fn monitor_lifetime(
    mut stream: BlockingLocalUnixStream,
    adoption: tokio::sync::oneshot::Sender<yolo_remote_broker::BrokerAdoptionAcknowledgement>,
    lost: tokio::sync::oneshot::Sender<()>,
    deadline: std::time::Instant,
) {
    let result = super::supervision_ack::set_deadline(&stream, deadline)
        .and_then(|_| read_adoption(&mut stream));
    if let Ok(nonce) = result {
        let (settled_send, settled_receive) = std::sync::mpsc::channel();
        let acknowledgement =
            super::supervision_ack::writer(&stream, nonce, deadline, settled_send);
        if let Ok(acknowledgement) = acknowledgement {
            if adoption.send(acknowledgement).is_ok() {
                if settled_receive.recv().unwrap_or(false) {
                    let _ = stream.set_read_timeout(None);
                    let _ = stream.set_write_timeout(None);
                    let mut unexpected = [u8::default(); 1];
                    let _ = stream.read(&mut unexpected);
                }
            }
        }
    }
    let _ = lost.send(());
}

fn read_adoption(stream: &mut BlockingLocalUnixStream) -> Result<uuid::Uuid> {
    match frame::read(stream).context(ADOPTION_READ_CONTEXT)? {
        Control::Adopt { nonce } => Ok(nonce),
        Control::Activate => anyhow::bail!(ADOPTION_MARKER_INVALID),
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/supervision_ancillary_support.rs"]
mod ancillary_support;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/supervision_ancillary_tests.rs"]
mod ancillary_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/supervision_activation_tests.rs"]
mod tests;
