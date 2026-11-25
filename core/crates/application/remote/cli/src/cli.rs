use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use yolo_cli::constants;
use yolo_remote_broker::BrokerLaunchDeadline;

#[derive(Debug, Parser)]
#[command(name = yolo_launch_contract::ProcessRole::Remote.display_name(), version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    Serve(ServerOptions),
    Start(ServerOptions),
    #[command(name = yolo_broker_launch_authority::START_HUB_COMMAND)]
    StartHub(super::options::HubStartOptions),
    #[command(name = constants::RETIRE_COMMAND)]
    Retire(super::options::RetireOptions),
    Status {
        #[arg(long)]
        socket: PathBuf,
    },
    #[command(name = constants::BOOTSTRAP_COMMAND)]
    Bootstrap {
        #[arg(long)]
        socket: PathBuf,
    },
    #[command(name = constants::WORKING_DIRECTORY_COMMAND)]
    WorkingDirectory {
        #[arg(long)]
        socket: PathBuf,
    },
    #[command(name = constants::REMOTE_COMMAND)]
    Exec(CommandOptions),
    ReadFile(FileOptions),
    #[command(name = constants::BROKER_SUPERVISOR_COMMAND, hide = true)]
    BrokerSupervisor(SupervisorOptions),
    #[command(name = constants::SUPERVISED_SERVE_COMMAND, hide = true)]
    SupervisedServe(SupervisedServeOptions),
}

#[derive(Clone, Debug, Args)]
pub(super) struct ServerOptions {
    #[arg(long)]
    pub(super) config: PathBuf,
    #[arg(long)]
    pub(super) state_dir: PathBuf,
    #[arg(long)]
    pub(super) expected_revision: Option<String>,
    #[arg(long)]
    pub(super) expected_anchor: Option<String>,
    #[arg(long)]
    pub(super) expected_authority: Option<String>,
    #[arg(long)]
    pub(super) expected_startup_deadline: Option<BrokerLaunchDeadline>,
    #[arg(long = constants::AUTHORITY_DOCUMENT_NAME)]
    pub(super) authority_document: Option<PathBuf>,
    #[arg(long = constants::AUTHORITY_LOCK_NAME)]
    pub(super) authority_lock: Option<PathBuf>,
    #[arg(long = constants::EXPECTED_PROFILE_SHA256_NAME)]
    pub(super) expected_profile_sha256: Option<String>,
    #[arg(long = constants::IDENTITY_PASSPHRASES_STDIN_NAME)]
    pub(super) identity_passphrases_stdin: bool,
}

#[derive(Debug, Args)]
pub(super) struct SupervisedServeOptions {
    #[command(flatten)]
    pub(super) server: ServerOptions,
    #[arg(long = constants::SUPERVISION_ACTIVATION_FD_NAME)]
    pub(super) activation_fd: i32,
}

#[derive(Debug, Args)]
pub(super) struct SupervisorOptions {
    #[command(flatten)]
    pub(super) server: ServerOptions,
    #[arg(long = constants::SUPERVISION_LAUNCHER_PID_NAME)]
    pub(super) launcher_pid: u32,
    #[arg(long = constants::SUPERVISION_LAUNCHER_START_TIME_NAME)]
    pub(super) launcher_start_time: u64,
}

#[derive(Debug, Args)]
pub(super) struct CommandOptions {
    #[arg(long)]
    pub(super) socket: PathBuf,
    #[arg(long)]
    pub(super) program: String,
    #[arg(long)]
    pub(super) cwd: Option<PathBuf>,
    #[arg(long, default_value_t = yolo_remote_broker::DEFAULT_COMMAND_TIMEOUT_MILLIS)]
    pub(super) timeout_millis: u64,
    #[arg(long, default_value_t = yolo_remote_broker::DEFAULT_COMMAND_OUTPUT_BYTES)]
    pub(super) output_limit: usize,
    #[arg(last = true)]
    pub(super) args: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct FileOptions {
    #[arg(long)]
    pub(super) socket: PathBuf,
    #[arg(long)]
    pub(super) path: PathBuf,
    #[arg(long, default_value_t = 0)]
    pub(super) offset: u64,
    #[arg(long, default_value_t = yolo_remote_broker::DEFAULT_FILE_READ_BYTES)]
    pub(super) length: u64,
}
