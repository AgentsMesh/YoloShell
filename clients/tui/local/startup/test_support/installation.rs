use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
};

pub(crate) struct Installation {
    pub root: tempfile::TempDir,
    pub executable: PathBuf,
}
impl Installation {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join(yolo_launch_contract::EXECUTABLE_NAME);
        fs::write(&executable, b"fixture").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        Self { root, executable }
    }
    pub fn alias(&self) -> PathBuf {
        let alias = self.root.path().join("tui-alias");
        symlink(&self.executable, &alias).unwrap();
        alias
    }
}
