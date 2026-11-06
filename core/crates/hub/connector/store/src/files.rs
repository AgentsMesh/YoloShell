use crate::{
    Store, StoredState,
    constants::*,
    record::{Index, Member, Record},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use yolo_hub_identity::ConnectorId;
use yolo_paths::{
    constants::MAX_DURABLE_JSON_BYTES, directory_stream::DirectoryStream, identity::FileIdentity,
};

impl<T: StoredState> Store<T> {
    pub(crate) fn member_path(&self, id: ConnectorId) -> PathBuf {
        self.members_path
            .join(id.to_string())
            .with_extension(MEMBER_EXTENSION)
    }
    pub(crate) fn check_index(&self) -> Result<()> {
        self.verify_namespace()?;
        let (index, identity): (Index<T>, _) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(
            identity == self.index_identity && index == self.index,
            REPLACED_INDEX
        );
        index.validate(self.directory, self.members_directory, self.identity)?;
        self.verify_namespace()
    }
    pub(crate) fn check_inventory(&self) -> Result<()> {
        self.verify_namespace()?;
        let directory = self.namespace.directory(&self.members_path)?;
        let mut stream = DirectoryStream::open(directory.descriptor())?;
        let allowed: BTreeSet<_> = self.index.members.iter().map(Member::id).collect();
        let mut observed = BTreeSet::new();
        let mut count = 0usize;
        while let Some(name) = stream.next_name()? {
            count = count.checked_add(1).context(CAPACITY)?;
            ensure!(count <= MAX_DIRECTORY_ENTRIES, CAPACITY);
            let path = Path::new(&name);
            if path
                .extension()
                .is_some_and(|extension| extension == MEMBER_EXTENSION)
            {
                let id: ConnectorId = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .context(UNINDEXED_MEMBER)?
                    .parse()?;
                ensure!(allowed.contains(&id), UNINDEXED_MEMBER);
                observed.insert(id);
            }
        }
        for member in &self.index.members {
            if matches!(member, Member::Ready { .. }) {
                ensure!(observed.contains(&member.id()), MEMBER_MISSING);
            }
        }
        self.verify_namespace()
    }
    pub(crate) fn load_member(&self, id: ConnectorId) -> Result<(Record<T>, FileIdentity)> {
        let (record, identity): (Record<T>, _) = self
            .namespace
            .read_json_with_identity(&self.member_path(id))?;
        record.validate(self.identity)?;
        let member = self.index.member(id).context(UNINDEXED_MEMBER)?;
        ensure!(
            record.seed.connector_id == id && record.seed.request_id == member.request_id(),
            MEMBER_MISMATCH
        );
        match member {
            Member::Preparing { initial } => ensure!(&record == initial, PREPARATION_MISMATCH),
            Member::Ready { seed_digest, .. } => {
                ensure!(record.seed.digest()? == *seed_digest, MEMBER_MISMATCH)
            }
        }
        self.verify_namespace()?;
        Ok((record, identity))
    }
    pub(crate) fn read_member(&self, id: ConnectorId) -> Result<Record<T>> {
        let expected = self.member_identities.get(&id).context(MEMBER_MISSING)?;
        let (record, identity) = self.load_member(id)?;
        ensure!(identity == *expected, REPLACED_MEMBER);
        Ok(record)
    }
    pub(crate) fn write_index(
        &mut self,
        next: Index<T>,
        member: Option<(ConnectorId, FileIdentity)>,
    ) -> Result<()> {
        next.validate(self.directory, self.members_directory, self.identity)?;
        let identity = self.namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(self.index_identity),
            &next,
            || {
                self.check_index()?;
                if let Some((id, expected)) = member {
                    self.namespace
                        .file(&self.member_path(id))?
                        .verify_exact_identity(expected, MAX_DURABLE_JSON_BYTES)?;
                }
                Ok(())
            },
        )?;
        self.index = next;
        self.index_identity = identity;
        self.check_index()
    }
}
