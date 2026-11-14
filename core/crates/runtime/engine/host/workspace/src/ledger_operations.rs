use crate::constants::*;
use crate::error::WorktreeError;
use crate::ledger::{Operation, Phase, WorktreeLedger};
use crate::{CreateWorktree, WorkspaceService, WorktreeRecord};
use std::path::{Path, PathBuf};

impl WorkspaceService {
    pub(crate) fn create_with_ledger(
        &self,
        request: &CreateWorktree,
    ) -> Result<WorktreeRecord, WorktreeError> {
        let (root, target) = self.git.target(request)?;
        let ledger = self.ledger()?;
        let token = format!("{CREATE_TOKEN_PREFIX}{}", request.worktree_id);
        ledger
            .reserve(token.clone(), Operation::Create, &root, &target, request.branch.clone())
            .map_err(|error| WorktreeError::Io(error.to_string()))?;
        match self.git.create(request) {
            Ok(record) => {
                ledger
                    .finish(&token, Phase::Linked)
                    .map_err(|_| WorktreeError::Unknown(LEDGER_FINISH_FAILED.into()))?;
                Ok(record)
            }
            Err(error) => {
                let phase = if retryable(&error) {
                    Phase::Unknown
                } else {
                    Phase::Failed
                };
                let _ = ledger.finish(&token, phase);
                Err(error)
            }
        }
    }

    pub(crate) fn remove_with_ledger(
        &self,
        root: &Path,
        target: &Path,
    ) -> Result<(), WorktreeError> {
        let ledger = self.ledger()?;
        let token = format!("{REMOVE_TOKEN_PREFIX}{}", target.display());
        ledger
            .reserve(
                token.clone(),
                Operation::Remove,
                root,
                target,
                None,
            )
            .map_err(|error| WorktreeError::Io(error.to_string()))?;
        match self.git.remove(root, target) {
            Ok(()) => {
                ledger
                    .finish(&token, Phase::Removed)
                    .map_err(|_| WorktreeError::Unknown(LEDGER_FINISH_FAILED.into()))?;
                Ok(())
            }
            Err(error) => {
                let phase = if retryable(&error) {
                    Phase::Unknown
                } else {
                    Phase::Failed
                };
                let _ = ledger.finish(&token, phase);
                Err(error)
            }
        }
    }

    /// Resolve unfinished attempts after a restart. This only appends evidence;
    /// it never removes a target or reissues git.
    pub(crate) fn reconcile_ledger(&self) {
        let Some(ledger) = self.ledger.as_ref() else {
            return;
        };
        for record in ledger.pending() {
            let root = Path::new(&record.root);
            let target = Path::new(&record.target);
            let Ok(listed) = self.git.list(root) else {
                continue;
            };
            let registered = listed
                .iter()
                .any(|worktree| same_path(Path::new(&worktree.path), target));
            let phase = match record.operation {
                Operation::Create if registered => Phase::Linked,
                Operation::Create if !target.exists() => Phase::Failed,
                Operation::Create => Phase::Unknown,
                Operation::Remove if !registered => Phase::Removed,
                Operation::Remove => Phase::Failed,
            };
            let _ = ledger.finish(&record.token, phase);
        }
    }

    fn ledger(&self) -> Result<&WorktreeLedger, WorktreeError> {
        self.ledger
            .as_ref()
            .ok_or_else(|| WorktreeError::Io(LEDGER_UNAVAILABLE.into()))
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn retryable(error: &WorktreeError) -> bool {
    matches!(
        error,
        WorktreeError::Timeout | WorktreeError::Unknown(_) | WorktreeError::Io(_)
    )
}
