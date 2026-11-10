use super::root_visit::Work;
use super::*;
use crate::paths::directory::DirectoryRevision;
use anyhow::Result;
use yolo_maintenance_contract::ScanChunk;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_certified_scan_lifecycle_tests.rs"]
mod certified_lifecycle_tests;
#[path = "root_scan/pending.rs"]
mod pending;

pub(super) struct RootScanner {
    root: PrivateDirectory,
    stream: Option<stream::DirectoryStream>,
    revision: Option<DirectoryRevision>,
    pending: Option<PendingStage>,
}

struct PendingStage {
    revision: DirectoryRevision,
    scanner: stage_scan::StageScanner,
    authority: root_visit::StageAuthority,
}

impl RootScanner {
    pub(super) fn new(root: PrivateDirectory) -> Self {
        Self {
            root,
            stream: None,
            revision: None,
            pending: None,
        }
    }

    pub(super) fn scan_chunk(&mut self) -> Result<ScanChunk> {
        if self.pending.is_some() {
            return self.scan_pending();
        }
        let Some(mut lifecycle) =
            lifecycle_lock::SocketNamespaceLifecycleLock::try_acquire(&self.root)?
        else {
            return Ok(ScanChunk::Busy);
        };
        self.prepare_stream()?;
        let work = self.scan_locked(&mut lifecycle);
        drop(lifecycle);
        let work = match work {
            Ok(work) => work,
            Err(error) => {
                self.reset();
                return Err(error);
            }
        };
        match work {
            Work::Complete => {
                self.reset();
                Ok(ScanChunk::PassComplete)
            }
            Work::Progress => {
                self.revision = Some(self.root.revision()?);
                Ok(ScanChunk::Progress)
            }
        }
    }

    fn prepare_stream(&mut self) -> Result<()> {
        self.root.verify_path()?;
        let revision = self.root.revision()?;
        if self.revision != Some(revision) {
            self.reset();
            self.revision = Some(revision);
        }
        if self.stream.is_none() {
            self.stream = Some(stream::DirectoryStream::open(self.root.descriptor())?);
        }
        Ok(())
    }

    fn scan_locked(
        &mut self,
        lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
    ) -> Result<Work> {
        for _ in 0..SOCKET_NAMESPACE_SWEEP_ENTRY_BUDGET {
            let Some(name) = self
                .stream
                .as_mut()
                .expect("root stream initialized")
                .next_name()?
            else {
                return Ok(Work::Complete);
            };
            if let Some(visit) = root_visit::visit(&self.root, &name, lifecycle)? {
                match visit {
                    root_visit::Visit::Pending {
                        revision,
                        directory,
                        authority,
                    } => {
                        self.pending = Some(PendingStage {
                            revision,
                            scanner: stage_scan::StageScanner::new(directory),
                            authority,
                        });
                        return Ok(Work::Progress);
                    }
                }
            }
        }
        Ok(Work::Progress)
    }

    pub(super) fn reset(&mut self) {
        self.pending = None;
        self.stream = None;
        self.revision = None;
    }
}
