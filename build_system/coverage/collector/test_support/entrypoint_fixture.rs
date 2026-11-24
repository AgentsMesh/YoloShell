#[path = "entrypoint_reports.rs"]
mod reports;
use crate::constants::*;
pub use crate::test_constants::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Fixture {
    directory: tempfile::TempDir,
    pub command: Command,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let coverage = root.join(FIXTURE_COVERAGE);
        std::fs::create_dir_all(&coverage).unwrap();
        std::fs::write(coverage.join(FIXTURE_RAW), FIXTURE_RAW).unwrap();
        let tool = PathBuf::from(std::env::var_os(FIXTURE_TOOL_ENV).unwrap())
            .canonicalize()
            .unwrap();
        for parent in [root.to_owned(), root.join(FIXTURE_RUNFILES)] {
            let path = parent.join(FIXTURE_TOOL);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&tool, path).unwrap();
        }
        for object in [FIXTURE_BINARY, FIXTURE_ADDITIONAL] {
            let path = root
                .join(FIXTURE_RUNFILES)
                .join(TEST_WORKSPACE)
                .join(object);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, object).unwrap();
        }
        reports::write(root);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([EXACT_TEST_ARGUMENT, CHILD_TEST, NO_CAPTURE_ARGUMENT])
            .env(FIXTURE_CHILD_ENV, CHILD_TEST)
            .env(FIXTURE_DIRECTORY_ENV, root)
            .env(EXECUTION_ROOT_ENV, root)
            .env(COVERAGE_DIRECTORY_ENV, &coverage)
            .env(RUNFILES_DIRECTORY_ENV, FIXTURE_RUNFILES)
            .env(TEST_WORKSPACE_ENV, TEST_WORKSPACE)
            .env(TEST_BINARY_ENV, FIXTURE_BINARY)
            .env(LLVM_COV_ENV, FIXTURE_TOOL)
            .env(LLVM_PROFDATA_ENV, FIXTURE_TOOL)
            .env(PREMATURE_EXIT_ENV, root.join(FIXTURE_MARKER))
            .env_remove(INSTRUMENTED_TEST_BINARY_ENV)
            .env_remove(ADDITIONAL_COVERAGE_OBJECTS_ENV)
            .env_remove(FIXTURE_FAILURE_ENV)
            .env_remove(COVERAGE_MANIFEST_ENV)
            .env_remove(VERBOSE_ENV);
        Self { directory, command }
    }

    pub fn root(&self) -> &Path {
        self.directory.path()
    }
    pub fn marker(&self) -> PathBuf {
        self.root().join(FIXTURE_MARKER)
    }
    pub fn report(&self) -> PathBuf {
        self.root()
            .join(FIXTURE_COVERAGE)
            .join(COVERAGE_REPORT_NAME)
    }
    pub fn profile(&self) -> PathBuf {
        self.root().join(FIXTURE_COVERAGE).join(PROFILE_DATA_NAME)
    }
    pub fn run(&mut self) -> Output {
        self.command.output().unwrap()
    }
    pub fn succeeds(&mut self) {
        let output = self.run();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            std::fs::read_to_string(self.report())
                .unwrap()
                .contains(FIXTURE_SOURCE)
        );
        assert!(!self.profile().exists());
    }
    pub fn fails(&mut self, message: &str) {
        let output = self.run();
        assert_eq!(output.status.code(), Some(FAILURE_EXIT_CODE));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(self.marker().exists());
        assert!(!self.report().is_file());
    }
}

pub fn partially_unreadable_inventory() -> Result<crate::environment::Context, String> {
    crate::environment::Context::load_with_profiles(|directory| {
        let raw = directory.join(FIXTURE_RAW);
        assert!(raw.is_file());
        crate::raw_profiles::select(
            directory,
            [
                Ok(raw),
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    ENTRY_READ_ERROR,
                )),
            ],
        )
    })
}
