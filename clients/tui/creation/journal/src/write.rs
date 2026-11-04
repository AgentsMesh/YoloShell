use crate::{
    CreationJournal,
    checkpoint::{self, Checkpoint},
    constants::*,
    record::Document,
};
use anyhow::{Context, Result, ensure};
use yolo_paths::identity::FileIdentity;

impl CreationJournal {
    pub(crate) fn install(&mut self, mut next: Document) -> Result<()> {
        next.revision = next
            .revision
            .checked_add(FIRST_REVISION)
            .context(REVISION_EXHAUSTED)?;
        next.validate(self.document.entry_hub, self.document.directory)?;
        let mut observed = None;
        let mut acknowledgement = Ok(());
        let result = self.namespace.write_json_expected_with_commit_observer(
            &self.path,
            Some(self.identity),
            &next,
            || {
                self.check_current()?;
                checkpoint::reach(Checkpoint::BeforeCommit)
            },
            |staged| {
                observed = self.observe_commit(&next, staged).ok();
                acknowledgement = checkpoint::reach(Checkpoint::CommitObserved);
            },
        );
        // A completed write supplies a fully verified identity even if an earlier
        // observer could not read it. An error admits only the observer's proof.
        if let Ok(identity) = &result {
            observed = Some(*identity);
        }
        self.recovery = observed.map(|identity| (next.clone(), identity));
        let identity = result?;
        acknowledgement?;
        checkpoint::reach(Checkpoint::WriteAcknowledged)?;
        self.identity = identity;
        self.document = next;
        self.check_current()?;
        self.recovery = None;
        Ok(())
    }

    fn observe_commit(&self, expected: &Document, staged: FileIdentity) -> Result<FileIdentity> {
        self.lock.verify()?;
        let (actual, identity): (Document, _) =
            self.namespace.read_json_with_identity(&self.path)?;
        // rename changes ctime. Capture the actual post-rename identity only
        // after proving the staged object/content and exact intended document.
        ensure!(
            identity.same_content_version(staged) && actual == *expected,
            CHANGED
        );
        self.lock.verify()?;
        self.namespace.verify()?;
        Ok(identity)
    }
}
