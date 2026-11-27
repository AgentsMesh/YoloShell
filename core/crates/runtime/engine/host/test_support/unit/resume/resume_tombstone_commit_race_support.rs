#[cfg(target_os = "macos")]
const CORRUPT_POST_CRASH_EXIT_TOMBSTONE: &[u8] = b"corrupt-tombstone";

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum PostCrashExitMutation {
    MatchingTombstone,
    ForeignEpochTombstone,
    CorruptTombstone,
    RuntimeReplacement,
}

#[cfg(target_os = "macos")]
impl PostCrashExitMutation {
    fn expected_block(self) -> AutomaticResumeBlock {
        match self {
            Self::MatchingTombstone => AutomaticResumeBlock::UserTermination,
            Self::ForeignEpochTombstone | Self::CorruptTombstone | Self::RuntimeReplacement => {
                AutomaticResumeBlock::RecoveryEvidenceChanged
            }
        }
    }

    fn apply(self, tombstone_path: &Path, runtime_path: &Path, runtime: &RuntimeRecord) {
        match self {
            Self::MatchingTombstone => {
                let tombstone = crate::model::TerminationTombstone::user_request(
                    runtime.terminal_id,
                    runtime.runtime_epoch,
                    true,
                );
                atomic_write_json(tombstone_path, &tombstone).unwrap();
            }
            Self::ForeignEpochTombstone => {
                let tombstone = crate::model::TerminationTombstone::user_request(
                    runtime.terminal_id,
                    Uuid::new_v4(),
                    true,
                );
                atomic_write_json(tombstone_path, &tombstone).unwrap();
            }
            Self::CorruptTombstone => {
                write_private_bytes(tombstone_path, CORRUPT_POST_CRASH_EXIT_TOMBSTONE);
            }
            Self::RuntimeReplacement => {
                let mut replacement = runtime.clone();
                replacement.worker_id = Uuid::new_v4();
                atomic_write_json(runtime_path, &replacement).unwrap();
            }
        }
    }
}
