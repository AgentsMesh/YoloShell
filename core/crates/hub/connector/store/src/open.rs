use crate::{
    Store, StoredState,
    constants::*,
    record::{Index, Member},
};
use anyhow::{Context, Result};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
use yolo_hub_delegation::PeerIdentity;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

impl<T: StoredState> Store<T> {
    pub fn open(directory: PrivateDirectory, identity: PeerIdentity) -> Result<Self> {
        let path = directory.path().join(INDEX_FILE);
        let members_path = directory.path().join(MEMBERS_DIRECTORY);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        let root_object = directory.durable_identity()?;
        let root =
            NamespaceLease::from_directories(vec![(ROOT_NAMESPACE, directory.try_clone()?)])?;
        let missing = root.file_is_missing(&path)?;
        let members = if missing {
            directory.ensure_descendant(&members_path)?
        } else {
            directory.open_descendant(&members_path)?
        };
        let members_directory = members.durable_identity()?;
        let namespace = NamespaceLease::from_directories(vec![
            (ROOT_NAMESPACE, directory),
            (MEMBER_NAMESPACE, members),
        ])?;
        let empty = Index {
            schema_version: SCHEMA_VERSION,
            identity,
            directory: root_object,
            members_directory,
            members: Vec::new(),
        };
        let (index, index_identity) = if missing {
            // Missing index plus existing members is loss of history, never adoption.
            Self::require_empty_members(&namespace, &members_path)?;
            let file =
                namespace
                    .write_json_expected_with_commit_fence(&path, None, &empty, || lock.verify())?;
            (empty, file)
        } else {
            namespace.read_json_with_identity(&path)?
        };
        index.validate(root_object, members_directory, identity)?;
        let mut store = Self {
            identity,
            directory: root_object,
            members_directory,
            path,
            members_path,
            namespace,
            lock,
            index,
            index_identity,
            member_identities: BTreeMap::new(),
            lost: AtomicBool::new(false),
        };
        store.check_index()?;
        store.check_inventory()?;
        // Validate all existing records before recovering any interrupted registration.
        let mut preparing = Vec::new();
        for member in &store.index.members {
            let id = member.id();
            let missing = store.namespace.file_is_missing(&store.member_path(id))?;
            if matches!(member, Member::Preparing { .. }) {
                preparing.push(id);
            } else {
                anyhow::ensure!(!missing, MEMBER_MISSING);
            }
            if !missing {
                let (_, file) = store.load_member(id)?;
                store.member_identities.insert(id, file);
            }
        }
        for id in preparing {
            store.stage_member(id)?;
            store.finish(id)?;
        }
        store.verify()?;
        Ok(store)
    }

    fn require_empty_members(namespace: &NamespaceLease, path: &std::path::Path) -> Result<()> {
        let directory = namespace.directory(path)?;
        let mut stream =
            yolo_paths::directory_stream::DirectoryStream::open(directory.descriptor())?;
        anyhow::ensure!(stream.next_name()?.is_none(), UNINDEXED_MEMBER);
        namespace.verify()
    }
}
