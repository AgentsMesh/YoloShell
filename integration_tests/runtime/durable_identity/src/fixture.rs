use crate::{
    constants::*,
    persistent_namespace::PersistentNamespace,
    snapshot::{Snapshot, device},
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use yolo_hub_configuration_store::{CONFIGURATION_DIRECTORY, ConfigurationStore};
use yolo_hub_state::{HUB_IDENTITY_FILE, HubState};
use yolo_paths::{Paths, directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub fn seed(root: &Path) -> Result<()> {
    let root_directory = PrivateDirectory::open(root)?;
    let namespace =
        NamespaceLease::from_directories(vec![(NAMESPACE, root_directory.try_clone()?)])?;
    ensure!(
        namespace.file_is_missing(&root.join(BASELINE_FILE))?,
        BASELINE_EXISTS
    );
    let runtime = root_directory.ensure_descendant(&root.join(RUNTIME))?;
    let runtime_namespace = Paths::new(runtime.path()).retain_socket_namespace()?;
    let hub = HubState::open(&root.join(HUB), runtime.path())?;
    let socket_namespaces = vec![
        PersistentNamespace::preserve(runtime_namespace.directory()?)?,
        PersistentNamespace::preserve(hub.socket_directory()?)?,
    ];
    let configuration_directory = hub
        .directory()?
        .ensure_descendant(&root.join(HUB).join(CONFIGURATION_DIRECTORY))?;
    let configuration =
        ConfigurationStore::open(configuration_directory, hub.peer_identity(), None)?;
    let creation = crate::creation::CreationEvidence::seed(root, hub.hub_id())?;
    let mut proofs = BTreeMap::new();
    for relative in [
        PathBuf::from(ROOT_PROOF),
        PathBuf::from(HUB),
        PathBuf::from(RUNTIME),
        PathBuf::from(crate::creation::DIRECTORY),
        PathBuf::from(HUB).join(CONFIGURATION_DIRECTORY),
    ] {
        let path = if relative == Path::new(ROOT_PROOF) {
            root.to_owned()
        } else {
            root.join(&relative)
        };
        proofs.insert(relative, PrivateDirectory::open(&path)?.durable_identity()?);
    }
    let mut records = BTreeMap::new();
    for relative in [
        PathBuf::from(HUB).join(HUB_IDENTITY_FILE),
        PathBuf::from(HUB).join(REGISTRATION_RECORD),
        PathBuf::from(RUNTIME).join(REGISTRATION_RECORD),
        PathBuf::from(crate::creation::DIRECTORY).join(yolo_tui_creation_journal::JOURNAL_FILE),
        PathBuf::from(HUB)
            .join(CONFIGURATION_DIRECTORY)
            .join(CONFIGURATION_RECORD),
    ] {
        records.insert(relative.clone(), std::fs::read(root.join(relative))?);
    }
    let snapshot = Snapshot {
        schema: BASELINE_SCHEMA,
        root: root.to_owned(),
        owner: unsafe { libc::geteuid() },
        device: device(root)?,
        peer: hub.peer_identity(),
        configuration: configuration.current()?,
        creation,
        proofs,
        records,
        socket_namespaces,
    };
    snapshot.verify()?;
    namespace.write_json(&root.join(BASELINE_FILE), &snapshot)?;
    runtime_namespace.verify()?;
    println!(
        "seed device={} hub={}",
        snapshot.device,
        snapshot.peer.hub()
    );
    Ok(())
}

pub fn reopen(root: &Path) -> Result<()> {
    let snapshot = Snapshot::load(root)?;
    let current_device = device(root)?;
    ensure!(snapshot.device != current_device, DEVICE_UNCHANGED);
    snapshot.verify()?;
    ensure!(
        HubState::inspect_peer_identity(&root.join(HUB), &root.join(RUNTIME))?
            == Some(snapshot.peer),
        IDENTITY_CHANGED
    );
    let runtime_namespace = Paths::new(root.join(RUNTIME)).retain_socket_namespace()?;
    let hub = HubState::open(&root.join(HUB), &root.join(RUNTIME))?;
    ensure!(hub.peer_identity() == snapshot.peer, IDENTITY_CHANGED);
    let directory = hub
        .directory()?
        .open_descendant(&root.join(HUB).join(CONFIGURATION_DIRECTORY))?;
    let configuration = ConfigurationStore::open(directory, hub.peer_identity(), None)?;
    ensure!(
        configuration.current()? == snapshot.configuration,
        IDENTITY_CHANGED
    );
    snapshot.verify()?;
    runtime_namespace.verify()?;
    println!(
        "reopen device_before={} device_after={} hub={}",
        snapshot.device,
        current_device,
        snapshot.peer.hub()
    );
    Ok(())
}
