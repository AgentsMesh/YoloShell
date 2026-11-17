use crate::{
    CommandChannel, Options, Outcome, RelayError, Status, constants::*, control::Control, output,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, watch},
    time::Instant,
};
use yolo_hub_client::Client;
use yolo_hub_terminal_protocol::{AttachRequest, TerminalSize};
use yolo_hub_terminal_client::Attachment;

pub async fn run(
    options: Options,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin,
    sizes: watch::Receiver<Option<TerminalSize>>,
    status: mpsc::Sender<Status>,
    cancel: impl std::future::Future<Output = ()>,
) -> Result<Outcome, RelayError> {
    run_inner(options, input, output, sizes, status, None, cancel).await
}

pub async fn run_with_commands(
    options: Options,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin,
    sizes: watch::Receiver<Option<TerminalSize>>,
    status: mpsc::Sender<Status>,
    commands: CommandChannel,
    cancel: impl std::future::Future<Output = ()>,
) -> Result<Outcome, RelayError> {
    if commands.launch_id.is_nil() {
        return Err(RelayError::InvalidOptions);
    }
    run_inner(
        options,
        input,
        output,
        sizes,
        status,
        Some(commands),
        cancel,
    )
    .await
}

async fn run_inner(
    options: Options,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin,
    sizes: watch::Receiver<Option<TerminalSize>>,
    status: mpsc::Sender<Status>,
    commands: Option<CommandChannel>,
    cancel: impl std::future::Future<Output = ()>,
) -> Result<Outcome, RelayError> {
    tokio::pin!(cancel);
    let mut control = tokio::select! {
        biased;
        _ = &mut cancel => return Ok(Outcome::Detached),
        result = Control::connect(options) => result?,
    };
    let result = {
        let operation = async {
            control.start().await?;
            let client = Client::connect_local(
                &control.options.socket,
                control.options.entry,
                Instant::now() + REQUEST_TIMEOUT,
            )
            .await?;
            let request = AttachRequest::new(control.options.target, None)
                .map_err(|_| RelayError::InvalidOptions)?;
            let attachment =
                Attachment::open(client, request, Instant::now() + REQUEST_TIMEOUT).await?;
            tokio::select! {
                result = output::stream(attachment, output, &status) => result,
                result = control.run(input, sizes, &status, commands) => result,
            }
        };
        tokio::select! {
            biased;
            _ = &mut cancel => Ok(Outcome::Detached),
            result = operation => result,
        }
    };
    control.release().await;
    result
}
