use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::{Command, Output};

pub(super) const ACTUAL_OUTPUT: &str = "bazel-out/k8-opt/bin/core/crates/application/yolo/yolo";
pub(super) const STALE_OUTPUT: &str = "bazel-out/k8-opt/bin/core/yolo";
pub(super) const QUERY_SUCCESS: i32 = 0;
pub(super) const QUERY_FAILURE: i32 = 7;
const QUERY_OUTPUT_ENVIRONMENT: &str = "YOLO_TEST_CQUERY_OUTPUT";
const QUERY_STATUS_ENVIRONMENT: &str = "YOLO_TEST_CQUERY_STATUS";
#[path = "linux_runtime_output_constants.rs"]
mod constants;
use constants::QUERY_FUNCTION;
const OUTPUT_TREE: &str = "output";
const LINK_PREFIX: &str = "bazel-";
const EXECUTABLE_BYTES: &[u8] = b"#!/bin/sh\nexit 0\n";
const SCRIPT_END: &str = "printf '%s' \"$runtime_executable\"";

pub(super) struct Fixture(tempfile::TempDir);

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(OUTPUT_TREE)).unwrap();
        symlink(root.path().join(OUTPUT_TREE), root.path().join("bazel-out")).unwrap();
        Self(root)
    }

    pub(super) fn executable(&self, output: &str) {
        let path = self.link_path(output);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, EXECUTABLE_BYTES).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE)).unwrap();
    }

    pub(super) fn link_path(&self, output: &str) -> PathBuf {
        self.0.path().join(output)
    }

    pub(super) fn remove_execute_permission(&self, output: &str) {
        fs::set_permissions(
            self.link_path(output),
            fs::Permissions::from_mode(PRIVATE_FILE_MODE),
        )
        .unwrap();
    }

    pub(super) fn replace_with_symlink(&self, output: &str, target: &str) {
        self.executable(target);
        fs::remove_file(self.link_path(output)).unwrap();
        symlink(self.link_path(target), self.link_path(output)).unwrap();
    }

    pub(super) fn capture(&self, output: &str, status: i32) -> Output {
        let prefix = self.0.path().join(LINK_PREFIX);
        let script = capture_script(BAZEL_BATCH_ARGUMENT, BAZEL_OPTIMIZED_ARGUMENT)
            .replace(DOCKER_BAZEL, QUERY_FUNCTION)
            .replace(DOCKER_SYMLINK_PREFIX, prefix.to_str().unwrap());
        let function = format!(
            "{QUERY_FUNCTION}() {{ printf '%s\\n' \"${QUERY_OUTPUT_ENVIRONMENT}\"; \
             return \"${QUERY_STATUS_ENVIRONMENT}\"; }}"
        );
        Command::new(CONTAINER_BASH)
            .args(CONTAINER_BASH_ARGUMENTS)
            .arg(format!("{function}\n{script}\n{SCRIPT_END}"))
            .env(QUERY_OUTPUT_ENVIRONMENT, output)
            .env(QUERY_STATUS_ENVIRONMENT, status.to_string())
            .output()
            .unwrap()
    }
}
