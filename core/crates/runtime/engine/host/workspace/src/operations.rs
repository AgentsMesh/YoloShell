use crate::command;
use crate::constants::*;
use crate::error::WorktreeError;
use crate::parse::parse;
use crate::validate::{repository_root, validate_branch, validate_id};
use crate::{CreateWorktree, GitWorkspace, WorktreeRecord, cleanup_failed_target, io};
use std::path::Path;

impl GitWorkspace {
    /// Resolve the repository root and the exact target path without effects.
    pub(crate) fn target(
        &self,
        request: &CreateWorktree,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), WorktreeError> {
        let executable = self
            .executable
            .as_deref()
            .ok_or_else(|| WorktreeError::Unavailable(self.unavailable.clone()))?;
        validate_id(&request.worktree_id)?;
        if let Some(branch) = &request.branch {
            validate_branch(branch)?;
        }
        if let Some(base) = &request.base {
            validate_branch(base)?;
        }
        let root = repository_root(executable, &request.repository_root)?;
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| WorktreeError::InvalidRepository(root.display().to_string()))?;
        let parent = root
            .parent()
            .ok_or_else(|| WorktreeError::InvalidRepository(root.display().to_string()))?;
        let container = parent.join(format!(".{name}.yolo-worktrees"));
        let target = container.join(&request.worktree_id);
        Ok((root, target))
    }

    pub fn create(&self, request: &CreateWorktree) -> Result<WorktreeRecord, WorktreeError> {
        if !request.trust_repository {
            return Err(WorktreeError::UntrustedRepository);
        }
        let executable = self
            .executable
            .as_deref()
            .ok_or_else(|| WorktreeError::Unavailable(self.unavailable.clone()))?;
        let (root, target) = self.target(request)?;
        if target.exists() {
            return self
                .find(&root, &target)
                .map_err(|_| WorktreeError::TargetExists(target.display().to_string()));
        }
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| WorktreeError::InvalidRepository(root.display().to_string()))?;
        let parent = root
            .parent()
            .ok_or_else(|| WorktreeError::InvalidRepository(root.display().to_string()))?;
        let container = parent.join(format!(".{name}.yolo-worktrees"));
        std::fs::create_dir_all(&container).map_err(io)?;
        let branch = request
            .branch
            .clone()
            .unwrap_or_else(|| format!("yolo/{}", request.worktree_id));
        let adopt = request.branch.is_none()
            && request.base.is_none()
            && command::run(
                executable,
                &[
                    GIT_REV_PARSE_ARGUMENT.to_owned(),
                    GIT_VERIFY_ARGUMENT.to_owned(),
                    format!("{GIT_BRANCH_REF_PREFIX}{branch}"),
                ],
                &root,
            )
            .is_ok();
        let mut args = if adopt {
            vec![
                "worktree".to_owned(),
                "add".to_owned(),
                target.display().to_string(),
                branch.clone(),
            ]
        } else {
            vec![
                "worktree".to_owned(),
                "add".to_owned(),
                "-b".to_owned(),
                branch.clone(),
                target.display().to_string(),
            ]
        };
        if let Some(base) = &request.base {
            args.push(base.clone());
        }
        let outcome = command::run(executable, &args, &root);
        match outcome {
            Ok(_) => {
                let record = self.find(&root, &target)?;
                Ok(record)
            }
            Err(error) => {
                cleanup_failed_target(&target);
                Err(error)
            }
        }
    }

    pub fn remove(&self, root: &Path, target: &Path) -> Result<(), WorktreeError> {
        let executable = self
            .executable
            .as_deref()
            .ok_or_else(|| WorktreeError::Unavailable(self.unavailable.clone()))?;
        let root = repository_root(executable, root)?;
        if !target.exists() {
            let registered = self
                .list(&root)?
                .iter()
                .any(|record| Path::new(&record.path) == target);
            if !registered {
                return Ok(());
            }
            command::run(
                executable,
                &[
                    GIT_WORKTREE_REMOVE_ARGUMENTS[0].to_owned(),
                    GIT_WORKTREE_REMOVE_ARGUMENTS[1].to_owned(),
                    GIT_FORCE_ARGUMENT.to_owned(),
                    target.display().to_string(),
                ],
                &root,
            )?;
            return Ok(());
        }
        self.find(&root, target)?;
        command::run(
            executable,
            &[
                "worktree".into(),
                "remove".into(),
                target.display().to_string(),
            ],
            &root,
        )?;
        Ok(())
    }

    pub fn list(&self, root: &Path) -> Result<Vec<WorktreeRecord>, WorktreeError> {
        let executable = self
            .executable
            .as_deref()
            .ok_or_else(|| WorktreeError::Unavailable(self.unavailable.clone()))?;
        let root = repository_root(executable, root)?;
        let output = command::run(
            executable,
            &crate::constants::GIT_WORKTREE_LIST_ARGUMENTS
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect::<Vec<_>>(),
            &root,
        )?;
        Ok(parse(&output))
    }

    pub(crate) fn find(&self, root: &Path, target: &Path) -> Result<WorktreeRecord, WorktreeError> {
        let target = std::fs::canonicalize(target).map_err(|_| {
            WorktreeError::Unknown("git reported success but the worktree is missing".into())
        })?;
        let found = self
            .list(root)?
            .into_iter()
            .find(|record| std::fs::canonicalize(&record.path).is_ok_and(|path| path == target));
        found.ok_or_else(|| WorktreeError::NotRegistered(target.display().to_string()))
    }
}
