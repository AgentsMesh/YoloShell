use super::lock_recover;
use super::pty_owner_handle::PtyOwnerCommand;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct PtyOwnerCommandSender {
    pub(crate) channel: mpsc::Sender<PtyOwnerCommand>,
    pub(crate) admission: Arc<Mutex<CommandAdmission>>,
}

pub(crate) struct PtyOwnerCommandReceiver {
    pub(crate) channel: mpsc::Receiver<PtyOwnerCommand>,
    pub(crate) admission: Arc<Mutex<CommandAdmission>>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum CommandAdmission {
    Open,
    Closed,
}

pub(crate) fn pty_owner_command_channel() -> (PtyOwnerCommandSender, PtyOwnerCommandReceiver) {
    let (sender, receiver) = mpsc::channel();
    let admission = Arc::new(Mutex::new(CommandAdmission::Open));
    (
        PtyOwnerCommandSender {
            channel: sender,
            admission: Arc::clone(&admission),
        },
        PtyOwnerCommandReceiver {
            channel: receiver,
            admission,
        },
    )
}

impl PtyOwnerCommandSender {
    #[cfg(test)]
    pub(crate) fn from_uncoordinated_sender(channel: mpsc::Sender<PtyOwnerCommand>) -> Self {
        Self {
            channel,
            admission: Arc::new(Mutex::new(CommandAdmission::Open)),
        }
    }

    pub(crate) fn send(
        &self,
        command: PtyOwnerCommand,
    ) -> Result<(), mpsc::SendError<PtyOwnerCommand>> {
        self.send_after_admission(command, || {})
    }

    pub(crate) fn send_after_admission(
        &self,
        command: PtyOwnerCommand,
        after_admission: impl FnOnce(),
    ) -> Result<(), mpsc::SendError<PtyOwnerCommand>> {
        let admission = lock_recover(&self.admission);
        if *admission == CommandAdmission::Closed {
            return Err(mpsc::SendError(command));
        }
        after_admission();
        self.channel.send(command)
    }

    #[cfg(test)]
    pub(crate) fn send_after_admission_for_test(
        &self,
        command: PtyOwnerCommand,
        after_admission: impl FnOnce(),
    ) -> Result<(), mpsc::SendError<PtyOwnerCommand>> {
        self.send_after_admission(command, after_admission)
    }
}

impl PtyOwnerCommandReceiver {
    #[cfg(test)]
    pub(crate) fn from_uncoordinated_receiver(channel: mpsc::Receiver<PtyOwnerCommand>) -> Self {
        Self {
            channel,
            admission: Arc::new(Mutex::new(CommandAdmission::Open)),
        }
    }

    pub(crate) fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<PtyOwnerCommand, mpsc::RecvTimeoutError> {
        self.channel.recv_timeout(timeout)
    }

    pub(crate) fn close_and_drain(self) -> Vec<PtyOwnerCommand> {
        self.close_and_drain_after(|| {})
    }

    pub(crate) fn close_and_drain_after(self, after_drain: impl FnOnce()) -> Vec<PtyOwnerCommand> {
        let mut admission = lock_recover(&self.admission);
        *admission = CommandAdmission::Closed;
        let commands = self.channel.try_iter().collect();
        after_drain();
        drop(self.channel);
        drop(admission);
        commands
    }

    #[cfg(test)]
    pub(crate) fn close_and_drain_after_for_test(
        self,
        after_drain: impl FnOnce(),
    ) -> Vec<PtyOwnerCommand> {
        self.close_and_drain_after(after_drain)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_command_channel_tests.rs"]
mod tests;
