use super::*;
use anyhow::Result;
use yolo_maintenance_contract::ScanChunk;

pub(super) struct StageScanner {
    directory: PrivateDirectory,
    target_parent: PathBuf,
    stream: Option<stream::DirectoryStream>,
    entries_seen: usize,
    targets_seen: usize,
}

impl StageScanner {
    pub(super) fn new(directory: PrivateDirectory) -> Self {
        let target_parent = directory.path().to_path_buf();
        Self {
            directory,
            target_parent,
            stream: None,
            entries_seen: 0,
            targets_seen: 0,
        }
    }

    pub(super) fn scan_chunk(&mut self) -> Result<ScanChunk> {
        if self.stream.is_none() {
            self.stream = Some(stream::DirectoryStream::open(self.directory.descriptor())?);
        }
        let mut cleanups = 0_usize;
        for _ in 0..SOCKET_STAGE_SWEEP_ENTRY_BUDGET {
            let Some(name) = self
                .stream
                .as_mut()
                .expect("stage stream initialized")
                .next_name()?
            else {
                // Directory retirement separately requires exclusive participant ownership.
                self.reset();
                return Ok(ScanChunk::PassComplete);
            };
            if self.entries_seen >= MAX_PRIVATE_DIRECTORY_ENTRIES {
                self.reset();
                return Ok(ScanChunk::Busy);
            }
            self.entries_seen += 1;
            let name = crate::paths::directory::entry_c_string(&name)?;
            let key = socket_stage::lease_key(&name);
            if key.is_none() && !socket_stage::cleanup::is_unlink_isolation(&name) {
                continue;
            }
            if self.targets_seen >= SOCKET_STAGE_TARGET_LIMIT {
                self.reset();
                return Ok(ScanChunk::PassComplete);
            }
            self.targets_seen += 1;
            let outcome = match key {
                Some(key) => {
                    socket_stage::cleanup_key_for_parent(&self.directory, key, &self.target_parent)?
                }
                None => socket_stage::cleanup::recover_unlink_isolation(&self.directory, &name)?,
            };
            match outcome {
                socket_stage::Cleanup::Busy
                | socket_stage::Cleanup::Unchanged
                | socket_stage::Cleanup::Unproven => {}
                socket_stage::Cleanup::Mutated => {
                    cleanups += 1;
                    if cleanups >= SOCKET_STAGE_CLEANUP_BUDGET {
                        self.restart_stream();
                        return Ok(ScanChunk::Progress);
                    }
                }
            }
        }
        Ok(ScanChunk::Progress)
    }

    pub(super) fn reset(&mut self) {
        self.restart_stream();
        self.entries_seen = 0;
        self.targets_seen = 0;
    }

    fn restart_stream(&mut self) {
        self.stream = None;
    }
}
