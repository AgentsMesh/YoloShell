use crate::{constants::*, snapshot::Snapshot};
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_hub_configuration_store::{CONFIGURATION_DIRECTORY, ConfigurationStore};
use yolo_hub_state::{HUB_IDENTITY_FILE, HubState};
use yolo_paths::directory::PrivateDirectory;

pub fn exercise(root: &Path) -> Result<()> {
    let snapshot = Snapshot::load(root)?;
    snapshot.verify()?;
    let parent = PrivateDirectory::open(root)?;
    let copied = parent.ensure_descendant(&root.join(CLONE))?;
    std::fs::copy(
        root.join(HUB)
            .join(CONFIGURATION_DIRECTORY)
            .join(CONFIGURATION_RECORD),
        copied.path().join(CONFIGURATION_RECORD),
    )?;
    ensure!(
        ConfigurationStore::open(copied, snapshot.peer, None).is_err(),
        COPY_ACCEPTED
    );
    let hub_path = root.join(HUB);
    std::fs::rename(&hub_path, root.join(RETIRED))?;
    parent.ensure_descendant(&hub_path)?;
    std::fs::copy(
        root.join(RETIRED).join(HUB_IDENTITY_FILE),
        hub_path.join(HUB_IDENTITY_FILE),
    )?;
    ensure!(
        HubState::inspect_peer_identity(&hub_path, &root.join(RUNTIME)).is_err(),
        REPLACEMENT_ACCEPTED
    );
    ensure!(
        HubState::open(&hub_path, &root.join(RUNTIME)).is_err(),
        REPLACEMENT_ACCEPTED
    );
    std::fs::rename(&hub_path, root.join(REJECTED))?;
    std::fs::rename(root.join(RETIRED), &hub_path)?;
    snapshot.verify()?;
    println!("negative copied_configuration=rejected replaced_hub=rejected");
    Ok(())
}
