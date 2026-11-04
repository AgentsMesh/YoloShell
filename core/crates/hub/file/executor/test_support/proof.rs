use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Proof {
    pub pid: u32,
    pub start: u64,
}

impl Proof {
    pub fn capture() -> Result<Self> {
        let pid = std::process::id();
        Ok(Self {
            pid,
            start: yolo_process::process_start_time(pid)?,
        })
    }
    pub fn write(self, directory: &Path) -> Result<()> {
        std::fs::write(
            directory.join(crate::constants::PROOF_FILE),
            serde_json::to_vec(&self)?,
        )?;
        Ok(())
    }
    pub fn gone(self) -> bool {
        yolo_process::observe_process_identity(self.pid, self.start)
            == yolo_process::ProcessIdentityObservation::Gone
    }
}
