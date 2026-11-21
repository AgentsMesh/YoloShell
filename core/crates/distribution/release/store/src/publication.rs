use crate::{
    InstalledBundle, ReleaseStore, Selection,
    checkpoint::{self, Checkpoint},
    constants::*,
    inventory,
    release::VerifiedRelease,
};
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use uuid::Uuid;
use yolo_distribution_bundle::BundleDescriptor;
use yolo_paths::{coordination::CoordinationLock, private_file::rename_exclusive_between};

impl ReleaseStore {
    /// Errors can occur after an atomic filesystem mutation. Inspect exact operation/build
    /// identity before retrying; this method never deletes releases or caller-owned stages.
    pub fn publish(
        &self,
        stage: Uuid,
        descriptor: &BundleDescriptor,
        expected: Option<Selection>,
        operation: Uuid,
    ) -> Result<InstalledBundle> {
        self.publish_inner(stage, descriptor, expected, operation)
            .context(PUBLICATION_UNCERTAIN)
    }

    fn publish_inner(
        &self,
        stage: Uuid,
        descriptor: &BundleDescriptor,
        expected: Option<Selection>,
        operation: Uuid,
    ) -> Result<InstalledBundle> {
        ensure!(!stage.is_nil() && !operation.is_nil(), INVALID_OPERATION);
        let lock = self.lock()?;
        let observed = self.read_selection()?;
        let current = observed.as_ref().map(|(selection, _)| selection);
        if let Some(selection) = current.filter(|selection| selection.operation_id() == operation) {
            ensure!(
                selection.build_id() == descriptor.build_id(),
                OPERATION_REUSE
            );
            let release = self.selected_release(selection)?;
            ensure!(release.descriptor == *descriptor, RELEASE_MISMATCH);
            release.sync()?;
            self.releases.sync()?;
            self.root.sync()?;
            self.verify_selection(&observed)?;
            self.verify(&lock)?;
            return Ok(installed(selection.clone(), release));
        }
        ensure!(current == expected.as_ref(), CONFLICT);
        // Verify the predecessor before any new publication; corrupted selected releases
        // cannot be silently replaced as though their recorded identity remained trusted.
        if let Some(selection) = current {
            self.selected_release(selection)?.verify()?;
        }
        let selected = Selection::next(current, operation, descriptor.build_id())?;
        let release = self.publish_directory(stage, descriptor, &lock)?;
        let expected_identity = observed.as_ref().map(|(_, identity)| *identity);
        let selector = self.root.path().join(CURRENT_FILE_NAME);
        let selected_identity = self.namespace.write_json_expected_with_commit_fence(
            &selector,
            expected_identity,
            &selected,
            || {
                checkpoint::reach(Checkpoint::BeforeSelectionCommit)?;
                self.verify(&lock)?;
                release.verify()?;
                self.verify_selection(&observed)
            },
        )?;
        checkpoint::reach(Checkpoint::AfterSelectionCommit)?;
        self.verify(&lock)?;
        release.verify()?;
        self.verify_selection(&Some((selected.clone(), selected_identity)))?;
        Ok(installed(selected, release))
    }

    fn publish_directory(
        &self,
        stage: Uuid,
        descriptor: &BundleDescriptor,
        lock: &CoordinationLock,
    ) -> Result<VerifiedRelease> {
        let path = descriptor.release_directory(self.root.path());
        let release = if inventory::exists(&self.releases, descriptor.build_id())? {
            // Existing content is immutable. A valid prior publication needs no upload.
            VerifiedRelease::open(&self.releases, &path, Some(descriptor))?
        } else {
            let stage_path = self.stage_path(stage)?;
            let staged = VerifiedRelease::open(&self.root, &stage_path, Some(descriptor))?;
            checkpoint::reach(Checkpoint::StageVerified)?;
            staged.sync()?;
            checkpoint::reach(Checkpoint::BeforeReleaseRename)?;
            self.verify(lock)?;
            staged.verify()?;
            let name = stage_path.file_name().context(IDENTITY_CHANGED)?;
            let source = CString::new(name.as_encoded_bytes())?;
            let target = CString::new(descriptor.build_id())?;
            rename_exclusive_between(
                self.root.descriptor(),
                &source,
                self.releases.descriptor(),
                &target,
            )?;
            checkpoint::reach(Checkpoint::AfterReleaseRename)?;
            staged.rebind(&self.releases, &path)?
        };
        release.sync()?;
        self.releases.sync()?;
        self.root.sync()?;
        checkpoint::reach(Checkpoint::ReleaseSynced)?;
        self.verify(lock)?;
        release.verify()?;
        Ok(release)
    }
}

fn installed(selection: Selection, release: VerifiedRelease) -> InstalledBundle {
    InstalledBundle {
        selection,
        directory: release.directory.path().to_path_buf(),
        descriptor: release.descriptor,
    }
}
