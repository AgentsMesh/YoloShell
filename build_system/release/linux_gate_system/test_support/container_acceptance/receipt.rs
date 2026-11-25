use super::constants::{PLANNED, SCHEMA};
use anyhow::Result;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::PRIVATE_FILE_MODE;

#[derive(Serialize)]
pub struct Receipt {
    #[serde(skip)]
    file: File,
    pub schema: u16,
    pub phase: &'static str,
    pub image: String,
    pub engine_identity: String,
    pub container_name: String,
    pub container_id: Option<String>,
    pub snapshot_root: PathBuf,
    pub tool_root: PathBuf,
    pub command: Vec<String>,
    pub cli_error: Option<String>,
    pub failure: Option<String>,
}

impl Receipt {
    pub fn create(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(path)?;
        let mut receipt = Self {
            file,
            schema: SCHEMA,
            phase: PLANNED,
            image: String::new(),
            engine_identity: String::new(),
            container_name: String::new(),
            container_id: None,
            snapshot_root: PathBuf::new(),
            tool_root: PathBuf::new(),
            command: Vec::new(),
            cli_error: None,
            failure: None,
        };
        receipt.save()?;
        Ok(receipt)
    }

    pub fn advance(&mut self, phase: &'static str) -> Result<()> {
        self.phase = phase;
        self.save()
    }

    pub fn save(&mut self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&self)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(&bytes)?;
        self.file.sync_all()?;
        Ok(())
    }
}
