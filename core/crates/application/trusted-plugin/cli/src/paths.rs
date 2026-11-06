use std::path::PathBuf;

pub fn registry_paths() -> anyhow::Result<yolo_trusted_plugin_registry::RegistryPaths> {
    if let Some(root) = std::env::var_os("YOLO_PLUGIN_REGISTRY_ROOT") {
        let root = PathBuf::from(root);
        return Ok(yolo_trusted_plugin_registry::RegistryPaths::new(
            root.join("config"),
            root.join("data"),
            root.join("state"),
        ));
    }
    let profile = yolo_local_profile::current()?;
    let root = profile
        .state
        .parent()
        .ok_or_else(|| anyhow::anyhow!("local profile state has no parent"))?;
    Ok(yolo_trusted_plugin_registry::RegistryPaths::new(
        root.join("plugins"),
        root.join("plugins-data"),
        root.join("plugins-state"),
    ))
}
