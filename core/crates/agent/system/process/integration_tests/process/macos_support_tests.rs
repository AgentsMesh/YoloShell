use super::*;
use anyhow::Result;
use std::cell::{Cell, RefCell};

pub(super) const ROOT: u32 = 10;
pub(super) const CHILD: u32 = 11;
pub(super) const GROUP: i32 = 20;

pub(super) struct MockApi {
    metadata: RefCell<HashMap<u32, Vec<ProcessMetadata>>>,
    pids: Vec<u32>,
    pub(super) expensive: RefCell<Vec<u32>>,
    executables: RefCell<Vec<PathBuf>>,
    cwds: RefCell<Vec<PathBuf>>,
    evidence_paths: RefCell<Vec<PathBuf>>,
    argv_readable: Cell<bool>,
    optional_reads: Cell<bool>,
}

impl MockApi {
    pub(super) fn new(entries: impl IntoIterator<Item = ProcessMetadata>, pids: Vec<u32>) -> Self {
        let metadata = entries
            .into_iter()
            .map(|entry| (entry.pid, vec![entry]))
            .collect();
        Self {
            metadata: RefCell::new(metadata),
            pids,
            expensive: RefCell::new(Vec::new()),
            executables: RefCell::new(vec![PathBuf::from("/usr/bin/codex")]),
            cwds: RefCell::new(vec![PathBuf::from("/workspace")]),
            evidence_paths: RefCell::new(Vec::new()),
            argv_readable: Cell::new(true),
            optional_reads: Cell::new(true),
        }
    }

    pub(super) fn push_metadata(&self, pid: u32, value: ProcessMetadata) {
        self.metadata
            .borrow_mut()
            .entry(pid)
            .or_default()
            .push(value);
    }

    pub(super) fn set_executables(&self, values: Vec<PathBuf>) {
        *self.executables.borrow_mut() = values;
    }

    pub(super) fn disable_optional_reads(&self) {
        self.optional_reads.set(false);
    }

    pub(super) fn disable_argv_read(&self) {
        self.argv_readable.set(false);
    }

    pub(super) fn set_evidence_paths(&self, values: Vec<PathBuf>) {
        *self.evidence_paths.borrow_mut() = values;
    }

    pub(super) fn set_cwds(&self, values: Vec<PathBuf>) {
        *self.cwds.borrow_mut() = values;
    }
}

impl ProcessApi for MockApi {
    fn metadata(&self, pid: u32) -> Result<ProcessMetadata> {
        let mut entries = self.metadata.borrow_mut();
        let values = entries
            .get_mut(&pid)
            .ok_or_else(|| anyhow::anyhow!("missing pid"))?;
        if values.len() > 1 {
            Ok(values.remove(0))
        } else {
            values
                .first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("missing metadata"))
        }
    }

    fn group_pids(&self, _group: i32, _limit: usize) -> Result<Vec<u32>> {
        Ok(self.pids.clone())
    }

    fn executable(&self, pid: u32) -> Result<PathBuf> {
        self.expensive.borrow_mut().push(pid);
        let mut paths = self.executables.borrow_mut();
        let path = if paths.len() > 1 {
            paths.remove(0)
        } else {
            paths
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing executable"))?
        };
        anyhow::ensure!(!path.as_os_str().is_empty(), "executable unavailable");
        Ok(path)
    }

    fn cwd(&self, _pid: u32) -> Result<PathBuf> {
        anyhow::ensure!(self.optional_reads.get(), "cwd unavailable");
        let mut paths = self.cwds.borrow_mut();
        if paths.len() > 1 {
            Ok(paths.remove(0))
        } else {
            paths
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing cwd"))
        }
    }

    fn argv(
        &self,
        _pid: u32,
        _byte_limit: usize,
        _environment_keys: &[&str],
    ) -> Result<ArgvSnapshot> {
        anyhow::ensure!(self.argv_readable.get(), "argv unavailable");
        Ok(ArgvSnapshot {
            values: vec!["codex".to_owned()],
            environment: Default::default(),
            bytes: 6,
        })
    }

    fn evidence_paths(&self, _pid: u32) -> Result<Vec<PathBuf>> {
        anyhow::ensure!(self.optional_reads.get(), "evidence paths unavailable");
        Ok(self.evidence_paths.borrow().clone())
    }
}

pub(super) fn entry(pid: u32, parent_pid: u32) -> ProcessMetadata {
    ProcessMetadata {
        pid,
        parent_pid,
        process_group: GROUP,
        foreground_group: GROUP,
        start_time: u64::from(pid),
    }
}
