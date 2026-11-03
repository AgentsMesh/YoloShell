use crate::{ObservationStore, Retirement, constants::*, index::Index, record::Record, size};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_routing::RoutingLimits;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, directory_stream::DirectoryStream,
    namespace_lease::NamespaceLease,
};

impl ObservationStore {
    pub fn open(
        directory: PrivateDirectory,
        entry: PeerIdentity,
        limits: RoutingLimits,
    ) -> Result<Self> {
        Self::open_with_retirement(directory, entry, limits, Retirement::default())
    }

    pub fn open_with_retirement(
        directory: PrivateDirectory,
        entry: PeerIdentity,
        limits: RoutingLimits,
        retirement: Retirement,
    ) -> Result<Self> {
        retirement.verify()?;
        let object = directory.durable_identity()?;
        let root = directory.path().to_path_buf();
        let path = root.join(OBSERVATION_INDEX_FILE);
        let lock_path = directory.path().join(LOCK_FILE);
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        namespace.verify()?;
        let lock = CoordinationLock::try_acquire(&lock_path, LOCK_BYTES)?.context(BUSY)?;
        retirement.verify()?;
        namespace.verify()?;
        let (index, index_file) = if namespace.file_is_missing(&path)? {
            let pinned = namespace.directory(&root)?;
            let mut stream = DirectoryStream::open(pinned.descriptor())?;
            while let Some(name) = stream.next_name()? {
                retirement.verify()?;
                ensure!(name == LOCK_FILE, UNINDEXED);
            }
            let index = Index {
                schema_version: SCHEMA_VERSION,
                directory: object,
                entry,
                origins: vec![],
            };
            size::validate(&index)?;
            let file =
                namespace.write_json_expected_with_commit_fence(&path, None, &index, || {
                    retirement.verify()?;
                    namespace.verify()?;
                    lock.verify()
                })?;
            (index, file)
        } else {
            namespace.read_json_with_identity::<Index>(&path)?
        };
        index.validate(object, entry, limits)?;
        let mut store = Self {
            index,
            records: BTreeMap::new(),
            root,
            path,
            namespace,
            lock,
            index_file,
            limits,
            lost: AtomicBool::new(false),
            retirement,
        };
        for origin in &store.index.origins {
            store.retirement.verify()?;
            let (record, file): (Record, _) = store
                .namespace
                .read_json_with_identity(&store.record_path(*origin))?;
            ensure!(record.catalogue.signer.hub() == *origin, CHANGED);
            record.history(object, entry, limits)?;
            store.records.insert(*origin, (record, file));
        }
        store.history()?;
        Ok(store)
    }
}
