#[derive(Debug, Args)]
struct CtlOptions {
    #[arg(long, required_unless_present = "existing_socket")]
    home: Option<PathBuf>,
    #[arg(long)]
    socket: Option<PathBuf>,
    #[arg(long, requires = "socket", conflicts_with = "existing_host")]
    existing_socket: bool,
    #[arg(
        long = yolo_runtime::cli::constants::EXISTING_HOST_NAME,
        requires = "home",
        conflicts_with_all = ["existing_socket", "socket"]
    )]
    existing_host: bool,
    #[command(subcommand)]
    command: CtlCommand,
}

#[derive(Debug, Args)]
struct LifecyclePreconditions {
    #[arg(
        long = yolo_runtime::cli::constants::EXPECTED_UPDATED_AT_NAME,
        requires = yolo_runtime::cli::constants::EXPECTED_STATUS_ID
    )]
    expected_updated_at_ms: Option<i64>,
    #[arg(
        long = yolo_runtime::cli::constants::EXPECTED_STATUS_NAME,
        requires = yolo_runtime::cli::constants::EXPECTED_UPDATED_AT_ID
    )]
    expected_status: Option<String>,
    #[arg(long = yolo_runtime::cli::constants::EXPECTED_MANIFEST_SHA256_NAME)]
    expected_manifest_sha256: Option<yolo_runtime::model::TerminalManifestDigest>,
    #[arg(long = yolo_runtime::cli::constants::EXPECTED_RUNTIME_EPOCH_NAME)]
    expected_runtime_epoch: Option<uuid::Uuid>,
}

#[derive(Debug, Subcommand)]
enum CtlCommand {
    Status,
    ShutdownIfIdle,
    List {
        #[arg(long)]
        include_archived: bool,
        #[arg(long = yolo_runtime::cli::constants::REQUIRE_COMPLETE_NAME)]
        require_complete: bool,
    },
    Create {
        #[command(flatten)]
        authorization: CreationAuthorization,
        #[arg(long = yolo_runtime::cli::constants::EXPORT_CREATION_AUTHORIZATION_NAME)]
        export_creation_authorization: bool,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        cwd: PathBuf,
    },
    Get {
        terminal_id: uuid::Uuid,
    },
    Snapshot {
        terminal_id: uuid::Uuid,
    },
    Attach {
        terminal_id: uuid::Uuid,
        #[arg(long, default_value_t = 0)]
        after_seq: u64,
        #[arg(long, default_value_t = 0)]
        events: usize,
    },
    Acquire {
        terminal_id: uuid::Uuid,
        #[arg(long)]
        holder: String,
        #[arg(long = yolo_runtime::cli::constants::EXPORT_WRITER_AUTHORIZATION_NAME)]
        export_writer_authorization: bool,
    },
    Takeover {
        terminal_id: uuid::Uuid,
        #[arg(long)]
        holder: String,
        #[arg(long = yolo_runtime::cli::constants::OBSERVED_GENERATION_NAME)]
        observed_generation: u64,
        #[arg(long = yolo_runtime::cli::constants::EXPORT_WRITER_AUTHORIZATION_NAME)]
        export_writer_authorization: bool,
    },
    Input {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: WriterAuthorization,
        #[arg(long)]
        data: String,
    },
    Resize {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: WriterAuthorization,
        #[arg(long)]
        cols: u16,
        #[arg(long)]
        rows: u16,
    },
    Signal {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: WriterAuthorization,
        signal: String,
    },
    Terminate {
        terminal_id: uuid::Uuid,
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        preconditions: LifecyclePreconditions,
    },
    Archive {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        preconditions: LifecyclePreconditions,
    },
    Discard {
        terminal_id: uuid::Uuid,
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        preconditions: LifecyclePreconditions,
    },
    CleanupTerminate {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: CreationAuthorization,
        #[arg(long)]
        force: bool,
    },
    CleanupArchive {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: CreationAuthorization,
    },
    CleanupDiscard {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: CreationAuthorization,
        #[arg(long)]
        force: bool,
    },
    FinalizeCreation {
        terminal_id: uuid::Uuid,
        #[command(flatten)]
        authorization: CreationAuthorization,
    },
    Resume {
        terminal_id: uuid::Uuid,
        #[arg(long)]
        recreate: bool,
        #[command(flatten)]
        preconditions: LifecyclePreconditions,
    },
    Reconcile,
}
