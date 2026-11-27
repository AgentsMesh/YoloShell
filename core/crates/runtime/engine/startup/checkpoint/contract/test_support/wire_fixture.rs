//! Serialize a validated current Rust checkpoint for native decoder contract tests.
mod wire_fixture_constants;
mod digest_fixture;

use anyhow::Result;
use uuid::Uuid;
use wire_fixture_constants::*;
use yolo_crypto::Sha256Digest;
use yolo_paths::identity::DurableFileVersion;
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence, StartupPhase, StartupProcessIdentity,
    StartupPtyCustodyRollover, StartupStage, StartupStatus, constants, validation,
};

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some(DIGEST_ARGUMENT) {
        return digest_fixture::emit();
    }
    let file = tempfile::NamedTempFile::new()?;
    let (source_version, _) =
        DurableFileVersion::capture_file_bounded(file.as_file(), file.path(), MAX_SOURCE_BYTES)?;
    let evidence = StartupCheckpointEvidence {
        schema_version: constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: Uuid::from_u128(TERMINAL),
        runtime_epoch: Uuid::from_u128(EPOCH),
        worker_id: Uuid::from_u128(WORKER),
        controller_generation: GENERATION,
        launch_sha256: Sha256Digest::repeated(LAUNCH_DIGEST),
        boot_id: BOOT.into(),
        worker: StartupProcessIdentity::new(WORKER_PID, WORKER_START),
        child: Some(StartupProcessIdentity::new(CHILD_PID, CHILD_START)),
        stage: StartupStage::SecondaryCustody,
        phase: StartupPhase::Entered,
        status: StartupStatus::Active,
        successor: None,
        predecessor_runtime_sha256: Some(Sha256Digest::repeated(PREDECESSOR_DIGEST)),
        pending_pty_custody_rollover: Some(StartupPtyCustodyRollover {
            source_version,
            source_sha256: Sha256Digest::repeated(SOURCE_DIGEST),
        }),
    };
    validation::validate(&evidence)?;
    serde_json::to_writer(std::io::stdout().lock(), &evidence)?;
    Ok(())
}
