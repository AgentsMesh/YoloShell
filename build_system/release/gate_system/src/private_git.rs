use crate::private_git_support::{
    execute, execute_output, pack_digest_from_path, private_arguments, validate_pack_digest,
};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use yolo_release_contracts::{GIT_DIRECTORY_NAME, SOURCE_ARCHIVE_NAME};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_process_execution::ProcessCommand;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{
    GIT_ARCHIVE_ARGUMENT, GIT_ARCHIVE_FORMAT_ARGUMENT, GIT_ARCHIVE_OUTPUT_PREFIX,
    GIT_CONFIG_ARGUMENT, GIT_DIRECTORY_PREFIX, GIT_EXECUTABLE, GIT_FSCK_ARGUMENTS,
    GIT_INDEX_PACK_ARGUMENTS, GIT_INIT_ARGUMENTS, GIT_METADATA_DIRECTORY_NAME,
    GIT_NO_REPLACE_ARGUMENT, GIT_NO_REVERSE_INDEX_CONFIG, GIT_OBJECT_FORMAT_PREFIX,
    GIT_PACK_DIRECTORY, GIT_PACK_EXTENSION, GIT_PACK_FILE_PREFIX, GIT_PACK_OBJECTS_ARGUMENTS,
    GIT_PACK_PREFIX, GIT_REV_LIST_ARGUMENTS, GIT_REVISION_LINE_ENDING,
};

pub(crate) struct PrivateGit {
    root: DirectoryHandle,
    git: DirectoryHandle,
    temporary: DirectoryHandle,
    revision: String,
    pack_relative_path: PathBuf,
}

impl PrivateGit {
    pub fn materialize(
        state: &WorkspaceState,
        root: &DirectoryHandle,
        git: &DirectoryHandle,
        temporary: &DirectoryHandle,
    ) -> Result<Self> {
        state.verify_root_identity()?;
        initialize(root, temporary, state.object_format())?;
        let digest = pack_objects(state, root, temporary)?;
        state.verify_root_identity()?;
        validate_pack_digest(&digest, state.object_format())?;
        let pack_relative_path =
            Path::new(GIT_DIRECTORY_NAME)
                .join(GIT_PACK_DIRECTORY)
                .join(format!(
                    "{GIT_PACK_FILE_PREFIX}{digest}{GIT_PACK_EXTENSION}"
                ));
        let private = Self {
            root: root.clone(),
            git: git.clone(),
            temporary: temporary.clone(),
            revision: state.revision().to_owned(),
            pack_relative_path,
        };
        private.ensure_complete()?;
        Ok(private)
    }

    pub fn export(&self) -> Result<()> {
        self.ensure_complete()?;
        let arguments = vec![
            format!("{GIT_DIRECTORY_PREFIX}{GIT_DIRECTORY_NAME}"),
            GIT_NO_REPLACE_ARGUMENT.to_owned(),
            GIT_ARCHIVE_ARGUMENT.to_owned(),
            GIT_ARCHIVE_FORMAT_ARGUMENT.to_owned(),
            format!("{GIT_ARCHIVE_OUTPUT_PREFIX}{SOURCE_ARCHIVE_NAME}"),
            self.revision.clone(),
        ];
        execute(ProcessCommand::new(
            GIT_EXECUTABLE,
            arguments,
            &self.root,
            &self.temporary,
            &self.temporary,
            "private Git archive export",
        ))?;
        self.ensure_complete()
    }

    pub fn ensure_complete(&self) -> Result<()> {
        self.verify_namespace()?;
        ensure!(
            !self
                .git
                .display_path()
                .join("objects/info/alternates")
                .exists(),
            "private Git database must not use alternates"
        );
        let pack = self.pack_relative_path.display().to_string();
        let index_arguments = private_arguments(GIT_INDEX_PACK_ARGUMENTS, [&pack]);
        let index_output = execute_output(ProcessCommand::new(
            GIT_EXECUTABLE,
            index_arguments,
            &self.root,
            &self.temporary,
            &self.temporary,
            "private Git index-pack validation",
        ))?;
        ensure!(
            String::from_utf8(index_output.stdout)?.trim()
                == pack_digest_from_path(&self.pack_relative_path)?,
            "private Git index-pack digest mismatch"
        );
        execute(ProcessCommand::new(
            GIT_EXECUTABLE,
            private_arguments(GIT_FSCK_ARGUMENTS, [&self.revision]),
            &self.root,
            &self.temporary,
            &self.temporary,
            "private Git fsck validation",
        ))?;
        execute(ProcessCommand::new(
            GIT_EXECUTABLE,
            private_arguments(GIT_REV_LIST_ARGUMENTS, [&self.revision]),
            &self.root,
            &self.temporary,
            &self.temporary,
            "private Git missing-object validation",
        ))?;
        self.verify_namespace()
    }

    fn verify_namespace(&self) -> Result<()> {
        self.root.verify_path()?;
        self.root.verify_child(GIT_DIRECTORY_NAME, &self.git)
    }

    #[cfg(test)]
    pub fn pack_path(&self) -> PathBuf {
        self.root.display_path().join(&self.pack_relative_path)
    }
}

fn initialize(root: &DirectoryHandle, temporary: &DirectoryHandle, format: &str) -> Result<()> {
    let mut arguments = GIT_INIT_ARGUMENTS
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    arguments.push(format!("{GIT_OBJECT_FORMAT_PREFIX}{format}"));
    arguments.push(GIT_DIRECTORY_NAME.to_owned());
    execute(ProcessCommand::new(
        GIT_EXECUTABLE,
        arguments,
        root,
        temporary,
        temporary,
        "private Git initialization",
    ))
}

fn pack_objects(
    state: &WorkspaceState,
    root: &DirectoryHandle,
    temporary: &DirectoryHandle,
) -> Result<String> {
    let prefix = Path::new(GIT_DIRECTORY_NAME)
        .join(GIT_PACK_DIRECTORY)
        .join(GIT_PACK_PREFIX)
        .display()
        .to_string();
    let mut arguments = vec![
        format!(
            "{GIT_DIRECTORY_PREFIX}{}",
            state.root().join(GIT_METADATA_DIRECTORY_NAME).display()
        ),
        GIT_NO_REPLACE_ARGUMENT.to_owned(),
        GIT_CONFIG_ARGUMENT.to_owned(),
        GIT_NO_REVERSE_INDEX_CONFIG.to_owned(),
    ];
    arguments.extend(
        GIT_PACK_OBJECTS_ARGUMENTS
            .iter()
            .map(|value| (*value).to_owned()),
    );
    arguments.push(prefix);
    let input = format!("{}{GIT_REVISION_LINE_ENDING}", state.revision()).into_bytes();
    let output = execute_output(
        ProcessCommand::new(
            GIT_EXECUTABLE,
            arguments,
            root,
            temporary,
            temporary,
            "fixed-revision Git pack materialization",
        )
        .with_input(input),
    )?;
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
