use super::constants::*;
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};
use yolo_local_profile::LocalProfile;

pub(super) struct Installation {
    root: Option<tempfile::TempDir>,
    pub tui: PathBuf,
    pub profile_path: PathBuf,
    pub workspace: PathBuf,
    pub profile: LocalProfile,
}

impl Installation {
    pub fn new(existing_host_home: Option<&Path>) -> Self {
        Self::with_image(existing_host_home, runfile(TUI_BINARY_ENV))
    }

    fn with_image(existing_host_home: Option<&Path>, image: PathBuf) -> Self {
        let root = tempfile::tempdir_in(TEST_TEMP_ROOT).unwrap();
        let directory = root.path().canonicalize().unwrap();
        let binaries = directory.join(BINARY_DIRECTORY);
        fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&binaries)
            .unwrap();
        let tui = binaries.join(yolo_launch_contract::EXECUTABLE_NAME);
        fs::copy(image, &tui).unwrap();
        let mut profile = LocalProfile::from_application_support(&directory).unwrap();
        if let Some(home) = existing_host_home {
            profile.host_home = home.canonicalize().unwrap();
        }
        profile.validate().unwrap();
        let profile_path = directory.join(PROFILE_FILE);
        yolo_runtime::paths::atomic_write_json(&profile_path, &profile).unwrap();
        Self {
            tui,
            profile_path,
            workspace: directory.join(WORKSPACE_DIRECTORY),
            root: Some(root),
            profile,
        }
    }

    pub fn remove_after_services_stopped(mut self) {
        self.root.take().unwrap().close().unwrap();
    }

    pub fn pending_bootstrap() -> Self {
        let mut installation = Self::with_image(None, runfile(BOOTSTRAP_FIXTURE_ENV));
        installation.profile.state = installation
            .profile
            .state
            .parent()
            .unwrap()
            .join(HOLD_DIRECTORY);
        installation.profile.validate().unwrap();
        yolo_runtime::paths::atomic_write_json(&installation.profile_path, &installation.profile)
            .unwrap();
        installation
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        // If startup or cleanup failed, preserve state that may belong to a
        // committed service. Never recursively remove uncertain service state.
        if let Some(root) = self.root.take() {
            let _retained = root.keep();
        }
    }
}

fn runfile(environment: &str) -> PathBuf {
    crate::bazel_runfile(&std::env::var(environment).unwrap()).unwrap()
}
