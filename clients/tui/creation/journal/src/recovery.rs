use crate::{CreationJournal, constants::CHANGED, record::Document};
use anyhow::{Result, ensure};
use std::sync::atomic::Ordering;
use yolo_paths::identity::FileIdentity;

impl CreationJournal {
    /// Recover an interrupted own operation without releasing its namespace or lock.
    /// A byte-identical replacement file is never a valid recovery candidate.
    pub fn recover(&mut self) -> Result<()> {
        let result = self.recover_current();
        self.retain_failure(result)
    }

    fn recover_current(&mut self) -> Result<()> {
        let (document, identity) = self.recovery_observation()?;
        let retained = document == self.document && identity == self.identity;
        let committed = self
            .recovery
            .as_ref()
            .is_some_and(|(expected, version)| document == *expected && identity == *version);
        ensure!(retained || committed, CHANGED);
        // The observer runs directly after rename, before parent fsync. Settle
        // that durability obligation before acknowledging the recovered write.
        self.namespace.file(&self.path)?.sync_parent()?;
        let (after, version) = self.recovery_observation()?;
        ensure!(after == document && version == identity, CHANGED);
        self.document = document;
        self.identity = identity;
        self.recovery = None;
        self.lost.store(false, Ordering::Release);
        Ok(())
    }

    fn recovery_observation(&self) -> Result<(Document, FileIdentity)> {
        self.lock.verify()?;
        self.namespace.verify()?;
        let (document, identity): (Document, _) =
            self.namespace.read_json_with_identity(&self.path)?;
        document.validate(self.document.entry_hub, self.document.directory)?;
        self.lock.verify()?;
        self.namespace.verify()?;
        Ok((document, identity))
    }
}
