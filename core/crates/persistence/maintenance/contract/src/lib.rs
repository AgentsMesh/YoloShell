use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanChunk {
    Busy,
    Progress,
    PassComplete,
}

pub trait MaintenanceScanner: Send {
    fn label(&self) -> &'static str;
    fn scan_chunk(&mut self) -> Result<ScanChunk>;
    fn reset(&mut self);
}

pub type ScannerSet = Vec<Box<dyn MaintenanceScanner>>;
