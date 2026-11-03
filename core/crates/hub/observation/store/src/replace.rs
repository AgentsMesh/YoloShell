use crate::checkpoint::{self, Checkpoint};
use crate::{ObservationStore, constants::*, record::Record, size};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_paths::constants::MAX_DURABLE_JSON_BYTES;

impl ObservationStore {
    /// Reject invalid/oversized caller snapshots without poisoning healthy retained storage.
    /// Any uncertain durable mutation permanently retires this instance.
    pub fn record_history(&mut self, history: &ObservationHistory) -> Result<()> {
        ensure!(history.entry() == self.index.entry, INVALID);
        history.validate(self.limits)?;
        self.history()?;
        let mut planned = Vec::new();
        let mut origins = self.index.origins.iter().copied().collect::<BTreeSet<_>>();
        for catalogue in history.catalogues() {
            let origin = catalogue.signer.hub();
            origins.insert(origin);
            if let Some((previous, _)) = self.records.get(&origin) {
                if previous.catalogue.signer == catalogue.signer {
                    let old = &previous.catalogue.payload;
                    let new = &catalogue.payload;
                    if old.revision == new.revision {
                        ensure!(old == new, CONFLICT);
                    }
                    if new.revision <= old.revision || new.policy_epoch < old.policy_epoch {
                        continue;
                    }
                }
            }
            let record = Record::new(self.index.directory, self.index.entry, catalogue);
            size::validate(&record)?;
            planned.push(record);
        }
        let mut next = self.index.clone();
        next.origins = origins.into_iter().collect();
        next.validate(self.index.directory, self.index.entry, self.limits)?;
        size::validate(&next)?;
        for record in planned {
            let result = self.write_origin(record);
            self.retain(result)?;
        }
        self.history().map(|_| ())
    }
    fn write_origin(&mut self, record: Record) -> Result<()> {
        let origin = record.catalogue.signer.hub();
        let path = self.record_path(origin);
        let expected = self.records.get(&origin).map(|(_, file)| *file);
        let result =
            self.namespace
                .write_json_expected_with_commit_fence(&path, expected, &record, || {
                    checkpoint::reach(Checkpoint::BeforeRecordCommit)?;
                    self.read().map(|_| ())
                });
        let file = result?;
        checkpoint::reach(Checkpoint::AfterRecordCommit)?;
        if expected.is_none() {
            let mut next = self.index.clone();
            next.origins.push(origin);
            next.origins.sort();
            next.validate(self.index.directory, self.index.entry, self.limits)?;
            let version = self.namespace.write_json_expected_with_commit_fence(
                &self.path,
                Some(self.index_file),
                &next,
                || {
                    self.read()?;
                    self.namespace
                        .file(&path)?
                        .verify_exact_identity(file, MAX_DURABLE_JSON_BYTES)
                },
            )?;
            checkpoint::reach(Checkpoint::AfterIndexCommit)?;
            self.index_file = version;
            self.index = next;
        }
        self.records.insert(origin, (record, file));
        self.history().map(|_| ())
    }
}
