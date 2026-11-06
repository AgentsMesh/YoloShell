use crate::{Arguments, CliError, command_frames, status_descriptor};
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, oneshot, watch},
};
use yolo_hub_terminal_relay::{
    COMMAND_QUEUE_CAPACITY, CommandChannel, ControlCommand, REQUEST_TIMEOUT,
};
use yolo_local_unix::LocalUnixStream;

pub(crate) struct CommandIo {
    socket: LocalUnixStream,
    outgoing: mpsc::Sender<ControlCommand>,
}

impl CommandIo {
    pub fn adopt(arguments: &Arguments) -> Result<Option<(Self, CommandChannel)>, CliError> {
        arguments.validate_channels()?;
        let Some(fd) = arguments.command_fd else {
            return Ok(None);
        };
        let socket = status_descriptor::adopt(fd).map_err(|_| CliError::CommandDescriptor)?;
        let (outgoing, incoming) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        Ok(Some((
            Self { socket, outgoing },
            CommandChannel {
                launch_id: arguments.launch_id.ok_or(CliError::CommandDescriptor)?,
                incoming,
            },
        )))
    }

    async fn serve(self) -> Result<(), CliError> {
        let (mut input, mut output) = tokio::io::split(self.socket);
        loop {
            let request = command_frames::read(&mut input).await?;
            let (response, received) = oneshot::channel();
            let pending = async {
                tokio::time::timeout(
                    REQUEST_TIMEOUT,
                    self.outgoing.send(ControlCommand {
                        request: request.clone(),
                        response,
                    }),
                )
                .await
                .map_err(|_| CliError::Command)?
                .map_err(|_| CliError::Command)?;
                tokio::time::timeout(REQUEST_TIMEOUT, received)
                    .await
                    .map_err(|_| CliError::Command)?
                    .map_err(|_| CliError::Command)
            };
            // One in-flight frame is allowed. EOF or pipelined input retires
            // the reply receiver, fencing queued work that has not dispatched.
            let response = tokio::select! {
                biased;
                _ = input.read_u8() => return Err(CliError::Command),
                response = pending => response?,
            };
            response
                .validate_for(&request)
                .map_err(|_| CliError::Command)?;
            command_frames::write(&mut output, &response).await?;
        }
    }
}

pub(crate) async fn run(
    io: Option<CommandIo>,
    cancel: watch::Sender<bool>,
    mut cancelled: watch::Receiver<bool>,
) -> Result<(), CliError> {
    let stopped = async {
        if !*cancelled.borrow_and_update() {
            let _ = cancelled.changed().await;
        }
    };
    let serve = async {
        match io {
            Some(io) => io.serve().await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        biased;
        _ = stopped => Ok(()),
        result = serve => { let _ = cancel.send(true); result }
    }
}
