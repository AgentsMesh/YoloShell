use super::super::constants::COMMAND_DATA_AFTER_EOF_MESSAGE;
use super::approved_channel::ApprovalBuffer;
use super::*;
use russh::ChannelMsg;
use tokio::time::Instant;

pub(super) async fn collect(
    channel: &mut russh::Channel<russh::client::Msg>,
    mut approved: ApprovalBuffer,
    command: &AuthorizedCommand,
    deadline: Instant,
) -> Result<CommandOutput, RemoteError> {
    let deadline = tokio::time::sleep_until(deadline);
    tokio::pin!(deadline);
    let mut collector = CommandCollector::new(command.request().output_limit());
    loop {
        tokio::select! {
            _ = &mut deadline => return Ok(collector.timed_out()),
            message = approved.next(channel) => {
                let Some(message) = message else {
                    return Ok(collector.finish());
                };
                if !collector.accept(message)? {
                    return Ok(collector.finish());
                }
            }
        }
    }
}

pub(super) fn timed_out() -> CommandOutput {
    let mut output = empty_output();
    output.timed_out = true;
    output
}

struct CommandCollector {
    output: CommandOutput,
    output_limit: usize,
    eof: bool,
}

impl CommandCollector {
    fn new(output_limit: usize) -> Self {
        Self {
            output: empty_output(),
            output_limit,
            eof: false,
        }
    }

    fn accept(&mut self, message: ChannelMsg) -> Result<bool, RemoteError> {
        match message {
            ChannelMsg::Data { data } => self.append_output(false, &data)?,
            ChannelMsg::ExtendedData { data, .. } => self.append_output(true, &data)?,
            ChannelMsg::ExitStatus { exit_status } => {
                self.output.exit_status = Some(exit_status);
            }
            ChannelMsg::Eof => self.eof = true,
            ChannelMsg::Close => return Ok(false),
            _ => {}
        }
        Ok(true)
    }

    fn append_output(&mut self, stderr: bool, data: &[u8]) -> Result<(), RemoteError> {
        if self.eof {
            return Err(RemoteError::transport(COMMAND_DATA_AFTER_EOF_MESSAGE));
        }
        let used = self
            .output
            .stdout
            .len()
            .saturating_add(self.output.stderr.len());
        let remaining = self.output_limit.saturating_sub(used);
        let target = if stderr {
            &mut self.output.stderr
        } else {
            &mut self.output.stdout
        };
        let accepted = remaining.min(data.len());
        target.extend_from_slice(&data[..accepted]);
        self.output.truncated |= accepted != data.len();
        Ok(())
    }

    fn timed_out(mut self) -> CommandOutput {
        self.output.timed_out = true;
        self.output
    }

    fn finish(self) -> CommandOutput {
        self.output
    }
}

fn empty_output() -> CommandOutput {
    CommandOutput {
        stdout: Vec::new(),
        stderr: Vec::new(),
        exit_status: None,
        timed_out: false,
        truncated: false,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/russh_backend/command_output_tests.rs"]
mod tests;
