use super::*;
use anyhow::Result;
use std::path::PathBuf;
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const SCANNER_LABEL: &str = "socket-namespace";
const RETAINED_ROOT_ERROR: &str = "socket root must remain retained";
const SCANNER_ROOT_ERROR: &str = "socket scanner root must remain retained";
const SCANNER_INITIALIZATION_ERROR: &str = "socket scanner must be initialized";

pub struct SocketNamespaceScanner {
    root_path: PathBuf,
    overridden_root: bool,
    retained_root: Option<PrivateDirectory>,
    scanner: Option<root_scan::RootScanner>,
}

impl SocketNamespaceScanner {
    pub fn new() -> Self {
        let effective_user_id = unsafe { libc::geteuid() };
        let override_path = std::env::var_os(SOCKET_ROOT_ENVIRONMENT).map(PathBuf::from);
        let selection =
            root_selection::SocketRootSelection::unchecked(effective_user_id, override_path);
        Self {
            root_path: selection.path,
            overridden_root: selection.overridden,
            retained_root: None,
            scanner: None,
        }
    }

    pub(crate) fn from_root(root: &PrivateDirectory, overridden_root: bool) -> Result<Self> {
        Ok(Self {
            root_path: root.path().to_path_buf(),
            overridden_root,
            retained_root: Some(root.try_clone()?),
            scanner: None,
        })
    }

    pub fn uses_override(&self) -> bool {
        self.overridden_root
    }

    pub fn owner_logical_path(&mut self) -> Result<PathBuf> {
        self.ensure_retained_root()?.verify_path()?;
        Ok(self.root_path.clone())
    }

    fn ensure_retained_root(&mut self) -> Result<&PrivateDirectory> {
        if self.retained_root.is_none() {
            self.retained_root = Some(root_selection::open(&self.root_path, self.overridden_root)?);
        }
        Ok(self.retained_root.as_ref().expect(RETAINED_ROOT_ERROR))
    }

    fn scanner(&mut self) -> Result<&mut root_scan::RootScanner> {
        self.ensure_retained_root()?;
        if self.scanner.is_none() {
            let retained = self.retained_root.as_ref().expect(SCANNER_ROOT_ERROR);
            retained.verify_path()?;
            self.scanner = Some(root_scan::RootScanner::new(retained.try_clone()?));
        }
        Ok(self.scanner.as_mut().expect(SCANNER_INITIALIZATION_ERROR))
    }
}

impl MaintenanceScanner for SocketNamespaceScanner {
    fn label(&self) -> &'static str {
        SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.scanner()?.scan_chunk()
    }

    fn reset(&mut self) {
        self.scanner = None;
    }
}
