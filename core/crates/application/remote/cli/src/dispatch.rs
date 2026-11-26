use crate::{Cli, Command, CommandOptions, FileOptions, conversion, error_constants, output};
use anyhow::Result;
use yolo_remote_broker::{BrokerClient, BrokerCommandRequest, BrokerFileRequest};

pub(super) async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Serve(options) => {
            let credentials = conversion::credentials(&options);
            yolo_broker_supervision::serve(conversion::child(options)?, credentials).await
        }
        Command::Start(options) => {
            let credentials = conversion::credentials(&options);
            let (request, authority) = conversion::start(options)?;
            output::print_start(yolo_broker_startup::start(request, authority, credentials).await?)
        }
        Command::StartHub(options) => {
            anyhow::ensure!(options.authority_stdio, error_constants::STDIO_REQUIRED);
            output::print_start(yolo_broker_startup::start_hub().await?)
        }
        Command::Retire(options) => yolo_broker_startup::retire(conversion::retire(options)).await,
        Command::Status { socket } => output::print(&BrokerClient::new(socket).status().await?),
        Command::Bootstrap { socket } => {
            output::print_bootstrap(BrokerClient::new(socket).bootstrap().await?)
        }
        Command::WorkingDirectory { socket } => {
            output::print_result(BrokerClient::new(socket).working_directory().await?)
        }
        Command::Exec(options) => command(options).await,
        Command::ReadFile(options) => read_file(options).await,
        Command::BrokerSupervisor(options) => {
            let credentials = conversion::credentials(&options.server);
            let launcher = yolo_process::ProcessIdentity {
                pid: options.launcher_pid,
                start_time: options.launcher_start_time,
            };
            yolo_broker_supervision::run_supervisor(
                conversion::child(options.server)?,
                credentials,
                launcher,
            )
            .await
        }
        Command::SupervisedServe(options) => {
            let credentials = conversion::credentials(&options.server);
            yolo_broker_supervision::run_supervised(
                conversion::child(options.server)?,
                credentials,
                options.activation_fd,
            )
            .await
        }
    }
}

async fn command(options: CommandOptions) -> Result<()> {
    let request = BrokerCommandRequest {
        program: options.program,
        args: options.args,
        cwd: options.cwd,
        timeout_millis: options.timeout_millis,
        output_limit: options.output_limit,
    };
    output::print_result(
        BrokerClient::new(options.socket)
            .run_command(request)
            .await?,
    )
}

async fn read_file(options: FileOptions) -> Result<()> {
    let request = BrokerFileRequest {
        path: options.path,
        offset: options.offset,
        length: options.length,
    };
    output::print_result(BrokerClient::new(options.socket).read_file(request).await?)
}
