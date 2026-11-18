use crate::private_git::PrivateGit;
use crate::requirements::GateCommand;
use crate::snapshot_support::utf8;
use anyhow::{Context, Result, ensure};
use std::fs;
use std::path::PathBuf;
use yolo_release_contracts::{
    BAZEL_LINK_NAME_PREFIX, GIT_DIRECTORY_NAME, HOME_DIRECTORY_NAME, LINKS_DIRECTORY_NAME,
    LINUX_ARTIFACT_DIRECTORY_NAME, OUTPUT_DIRECTORY_NAME, SNAPSHOT_PREFIX, SOURCE_ARCHIVE_NAME,
    SOURCE_DIRECTORY_NAME, TEMP_DIRECTORY_NAME,
};
use yolo_release_evidence_model::GateExecutionEvidence;
use yolo_release_filesystem::TreeManifest;
use yolo_release_filesystem::set_read_only;
use yolo_release_filesystem::{DirectoryHandle, TemporaryDirectory};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::TrustedBazel;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{
    SNAPSHOT_COMMAND_TIMEOUT, TAR_DIRECTORY_ARGUMENT, TAR_EXECUTABLE, TAR_EXTRACT_ARGUMENTS,
};

pub struct GateSnapshot {
    root_owner: TemporaryDirectory,
    pub(crate) root: DirectoryHandle,
    pub(crate) workspace: DirectoryHandle,
    pub(crate) output: DirectoryHandle,
    pub(crate) links: DirectoryHandle,
    pub(crate) home: DirectoryHandle,
    pub(crate) temporary: DirectoryHandle,
    pub(crate) git: DirectoryHandle,
    pub(crate) linux_artifacts: DirectoryHandle,
    pub(crate) source_manifest: Option<TreeManifest>,
    pub(crate) linux_artifact_manifest: Option<TreeManifest>,
}

impl GateSnapshot {
    pub fn create(state: &WorkspaceState) -> Result<Self> {
        let mut snapshot = Self::empty()?;
        let private_git =
            PrivateGit::materialize(state, &snapshot.root, &snapshot.git, &snapshot.temporary)?;
        private_git.export()?;
        snapshot.extract()?;
        fs::remove_file(snapshot.root.display_path().join(SOURCE_ARCHIVE_NAME))
            .context("remove private source archive")?;
        set_read_only(snapshot.workspace.display_path())?;
        snapshot.source_manifest = Some(TreeManifest::capture(&snapshot.workspace)?);
        snapshot.verify_integrity()?;
        Ok(snapshot)
    }

    fn extract(&self) -> Result<()> {
        let mut arguments = TAR_EXTRACT_ARGUMENTS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        arguments.extend([
            SOURCE_ARCHIVE_NAME.to_owned(),
            TAR_DIRECTORY_ARGUMENT.to_owned(),
            SOURCE_DIRECTORY_NAME.to_owned(),
        ]);
        let output = ProcessCommand::new(
            TAR_EXECUTABLE,
            arguments,
            &self.root,
            &self.temporary,
            &self.temporary,
            "private source archive extraction",
        )
        .with_timeout(SNAPSHOT_COMMAND_TIMEOUT)
        .execute()?;
        ensure!(
            output.status.success(),
            "release snapshot extraction failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    pub(crate) fn empty() -> Result<Self> {
        let mut root_owner = TemporaryDirectory::create(SNAPSHOT_PREFIX)?;
        let root = root_owner.handle().clone();
        let workspace = root_owner.create_child(SOURCE_DIRECTORY_NAME)?;
        let output = root_owner.create_child(OUTPUT_DIRECTORY_NAME)?;
        let links = root_owner.create_child(LINKS_DIRECTORY_NAME)?;
        let home = root_owner.create_child(HOME_DIRECTORY_NAME)?;
        let temporary = root_owner.create_child(TEMP_DIRECTORY_NAME)?;
        let git = root_owner.create_child(GIT_DIRECTORY_NAME)?;
        let linux_artifacts = root_owner.create_child(LINUX_ARTIFACT_DIRECTORY_NAME)?;
        Ok(Self {
            root_owner,
            root,
            workspace,
            output,
            links,
            home,
            temporary,
            git,
            linux_artifacts,
            source_manifest: None,
            linux_artifact_manifest: None,
        })
    }

    pub fn verify_integrity(&self) -> Result<()> {
        self.root.verify_path()?;
        for (name, handle) in self.children() {
            self.root.verify_child(name, handle)?;
        }
        let expected = self
            .source_manifest
            .as_ref()
            .context("release source manifest is not sealed")?;
        ensure!(
            TreeManifest::capture(&self.workspace)? == *expected,
            "release source manifest changed"
        );
        if let Some(expected) = &self.linux_artifact_manifest {
            ensure!(
                TreeManifest::capture(&self.linux_artifacts)? == *expected,
                "Linux artifact repository changed"
            );
        }
        Ok(())
    }

    pub fn execution(&self) -> Result<GateExecutionEvidence> {
        Ok(GateExecutionEvidence {
            workspace_snapshot: utf8(self.workspace.display_path())?,
            output_user_root: utf8(self.output.display_path())?,
            symlink_prefix: utf8(&self.links.display_path().join(BAZEL_LINK_NAME_PREFIX))?,
            isolated_home: utf8(self.home.display_path())?,
            isolated_tmp: utf8(self.temporary.display_path())?,
            linux_artifact_repository: utf8(self.linux_artifacts.display_path())?,
            namespace_identity: self.root.identity(),
            workspace_identity: self.workspace.identity(),
            output_identity: self.output.identity(),
            links_identity: self.links.identity(),
            home_identity: self.home.identity(),
            tmp_identity: self.temporary.identity(),
            linux_artifact_identity: self.linux_artifacts.identity(),
        })
    }

    pub fn command(&self, launcher: &TrustedBazel, gate: GateCommand) -> Result<ProcessCommand> {
        let execution = self.execution()?;
        Ok(ProcessCommand::new(
            launcher.path(),
            gate.arguments(&execution),
            &self.workspace,
            &self.home,
            &self.temporary,
            "bounded Bazel release command",
        )
        .retain_directories(launcher.resource_directory()))
    }

    pub fn root_path(&self) -> PathBuf {
        self.root_owner.path()
    }

    pub fn close(mut self) -> Result<()> {
        self.root_owner.cleanup().with_context(|| {
            format!(
                "release snapshot retained at {}",
                self.root_owner.path().display()
            )
        })
    }

    fn children(&self) -> [(&'static str, &DirectoryHandle); 7] {
        [
            (SOURCE_DIRECTORY_NAME, &self.workspace),
            (OUTPUT_DIRECTORY_NAME, &self.output),
            (LINKS_DIRECTORY_NAME, &self.links),
            (HOME_DIRECTORY_NAME, &self.home),
            (TEMP_DIRECTORY_NAME, &self.temporary),
            (GIT_DIRECTORY_NAME, &self.git),
            (LINUX_ARTIFACT_DIRECTORY_NAME, &self.linux_artifacts),
        ]
    }
}
