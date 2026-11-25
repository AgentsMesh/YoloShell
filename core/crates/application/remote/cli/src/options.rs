use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(super) struct HubStartOptions {
    #[arg(long = yolo_broker_launch_authority::AUTHORITY_STDIO_NAME, required = true)]
    pub(super) authority_stdio: bool,
}

#[derive(Clone, Debug, Args)]
pub(super) struct RetireOptions {
    #[arg(long)]
    pub(super) state_dir: PathBuf,
    #[arg(long = yolo_cli::constants::EXPECTED_HOST_NAME)]
    pub(super) expected_host: String,
    #[arg(long = yolo_cli::constants::AUTHORITY_DOCUMENT_NAME)]
    pub(super) authority_document: PathBuf,
    #[arg(long = yolo_cli::constants::AUTHORITY_LOCK_NAME)]
    pub(super) authority_lock: PathBuf,
    #[arg(long = yolo_cli::constants::EXPECTED_MUTATION_ID_NAME)]
    pub(super) expected_mutation_id: uuid::Uuid,
}
