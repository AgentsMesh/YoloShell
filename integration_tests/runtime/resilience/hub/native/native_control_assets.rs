use super::constants::*;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub struct Assets {
    pub binary: PathBuf,
    pub data: PathBuf,
    pub first: PathBuf,
    pub next: PathBuf,
    pub target_marker: PathBuf,
    pub other_marker: PathBuf,
}

impl Assets {
    pub fn prepare(home: &Path) -> Self {
        let home = home.canonicalize().unwrap();
        let bin = home.join(BINARY_DIRECTORY);
        std::fs::create_dir(&bin).unwrap();
        let binary = bin.join(yolo_runtime::agent::claude::constants::EXECUTABLE);
        std::fs::copy(
            crate::bazel_runfile(crate::FAKE_CLAUDE_BINARY_RUNFILE).unwrap(),
            &binary,
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(EXECUTABLE_MODE))
            .unwrap();
        let data = home.join(DATA_DIRECTORY);
        let provider = crate::support::claude_install_spec();
        let preview = yolo_hook_installation::inspect(provider, &data).unwrap();
        yolo_hook_installation::install(provider, &data, &preview.revision).unwrap();
        let first = home.join(FIRST_DIRECTORY);
        let next = home.join(NEXT_DIRECTORY);
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&next).unwrap();
        Self {
            binary,
            data,
            first,
            next,
            target_marker: home.join(TARGET_MARKER),
            other_marker: home.join(OTHER_MARKER),
        }
    }
}

pub fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', SHELL_QUOTE_ESCAPE)
}
