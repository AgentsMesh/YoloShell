use std::path::PathBuf;
use yolo_paths::directory::PrivateDirectory;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryPaths {
    pub config: PathBuf,
    pub data_root: PathBuf,
    pub state_root: PathBuf,
}

impl RegistryPaths {
    pub fn new(
        config: impl Into<PathBuf>,
        data: impl Into<PathBuf>,
        state: impl Into<PathBuf>,
    ) -> Self {
        Self {
            config: config.into(),
            data_root: data.into(),
            state_root: state.into(),
        }
    }

    pub fn registry_file(&self) -> PathBuf {
        self.config.join("plugins.json")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.config.join("plugins.lock")
    }

    pub fn data_dir(&self, id: &str) -> PathBuf {
        self.data_root.join(id)
    }

    pub fn log_dir(&self, id: &str) -> PathBuf {
        self.state_root.join(id).join("logs")
    }

    pub fn ensure(&self) -> anyhow::Result<()> {
        for path in [&self.config, &self.data_root, &self.state_root] {
            PrivateDirectory::ensure(path)?;
        }
        Ok(())
    }
}
