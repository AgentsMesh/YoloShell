use crate::{
    Arguments, CliError,
    command_io::{self, CommandIo},
    status::Reporter,
    status_descriptor,
};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::{mpsc, oneshot, watch},
};
use yolo_hub_terminal_io::{Descriptor, RawTerminal, terminal_size};
use yolo_hub_terminal_relay::{CommandChannel, Options, STATUS_QUEUE_CAPACITY, Status};

pub async fn run(arguments: Arguments) -> Result<(), CliError> {
    // Validate disjoint descriptors before taking ownership of either number.
    arguments.validate_channels()?;
    let (commands, incoming) = match CommandIo::adopt(&arguments)? {
        Some((io, channel)) => (Some(io), Some(channel)),
        None => (None, None),
    };
    let _raw = RawTerminal::enable(libc::STDIN_FILENO)?;
    let options = arguments.options(terminal_size(libc::STDIN_FILENO)?);
    let (input, output, reporter) = match arguments.status_fd {
        Some(fd) => {
            let reporter = Reporter::Socket(status_descriptor::adopt(fd)?);
            let (input, output) = Descriptor::pair(libc::STDIN_FILENO, libc::STDOUT_FILENO)?;
            (input, output, reporter)
        }
        None => {
            let (input, output, stderr) =
                Descriptor::triple(libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO)?;
            (input, output, Reporter::Stderr(stderr))
        }
    };
    let (status, statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let (cancel, cancelled) = watch::channel(false);
    let command_runner = command_io::run(commands, cancel.clone(), cancelled.clone());
    let report = async {
        let result = reporter.run(statuses).await;
        let _ = cancel.send(true);
        result
    };
    let (relay, report, commands) = tokio::join!(
        relay(options, input, output, status, incoming, cancelled),
        report,
        command_runner
    );
    relay?;
    commands?;
    report
}

async fn relay(
    options: Options,
    input: Descriptor,
    output: Descriptor,
    status: mpsc::Sender<Status>,
    commands: Option<CommandChannel>,
    mut cancelled: watch::Receiver<bool>,
) -> Result<(), CliError> {
    let mut resize = signal(SignalKind::window_change())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let (size, sizes) = watch::channel(options.size);
    let (failed, failure) = oneshot::channel::<()>();
    let mut failed = Some(failed);
    let cancellation = async {
        let parent = async {
            if !*cancelled.borrow_and_update() {
                let _ = cancelled.changed().await;
            }
        };
        tokio::select! {
            _ = parent => {}, _ = interrupt.recv() => {}, _ = terminate.recv() => {}, _ = hangup.recv() => {},
            _ = failure => {},
        }
    };
    let relay = async {
        match commands {
            Some(commands) => {
                yolo_hub_terminal_relay::run_with_commands(
                    options,
                    input,
                    output,
                    sizes,
                    status,
                    commands,
                    cancellation,
                )
                .await
            }
            None => {
                yolo_hub_terminal_relay::run(options, input, output, sizes, status, cancellation)
                    .await
            }
        }
    };
    tokio::pin!(relay);
    loop {
        tokio::select! {
            result = &mut relay => { result?; return Ok(()); },
            _ = resize.recv() => match terminal_size(libc::STDIN_FILENO) {
                Ok(Some(current)) => { let _ = size.send(Some(current)); },
                Ok(None) => {},
                Err(error) => {
                    let _ = failed.take().unwrap().send(());
                    let _ = relay.await;
                    return Err(error.into());
                }
            },
        }
    }
}
