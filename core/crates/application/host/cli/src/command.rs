#[derive(Debug, Subcommand)]
enum Command {
    Hooks(hook_installation::Options),
    Serve {
        #[arg(long)]
        home: PathBuf,
        #[arg(long)]
        socket: Option<PathBuf>,
        #[arg(long = yolo_runtime::cli::constants::DETACHED_SESSION_NAME, hide = true)]
        detached_session: bool,
        #[arg(long = yolo_startup_guard::ARGUMENT_NAME, hide = true)]
        startup_guard: bool,
    },
    Bridge {
        #[arg(long)]
        stdio: bool,
        #[arg(long)]
        home: PathBuf,
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    CatalogStream(catalog_stream::CatalogStreamOptions),
    SessionWorker {
        #[arg(long)]
        home: PathBuf,
        #[arg(long)]
        terminal_id: uuid::Uuid,
    },
    #[command(hide = true)]
    CustodyGuardian {
        #[arg(long)]
        home: PathBuf,
        #[arg(long)]
        terminal_id: uuid::Uuid,
    },
    #[command(hide = true)]
    BootstrapChild,
    #[command(
        hide = true,
        name = yolo_runtime::cli::constants::PROVIDER_HOOK_COMMAND
    )]
    ProviderHook {
        #[arg(long)]
        provider: String,
        #[arg(long)]
        home: PathBuf,
    },
    #[command(hide = true)]
    ProcessSnapshot {
        #[arg(long)]
        root_pid: u32,
        #[arg(long)]
        root_start_time: u64,
        #[arg(
            long = yolo_runtime::cli::constants::PARENT_WATCH_NAME,
            hide = true
        )]
        parent_watch: bool,
        #[arg(
            long = yolo_runtime::cli::constants::ENVIRONMENT_KEY_NAME,
            hide = true
        )]
        environment_keys: Vec<String>,
    },
    Ctl(CtlOptions),
}
