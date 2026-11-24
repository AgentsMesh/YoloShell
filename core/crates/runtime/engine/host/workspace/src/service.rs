use crate::error::WorktreeError;
use crate::ledger::{WORKTREE_LEDGER_FILE, WorktreeLedger};
use crate::{CreateWorktree, GitWorkspace};
use serde::Serialize;
use serde::de::DeserializeOwned;
use yolo_runtime_protocol::{
    GitProbe, WorkspaceInspectRequest, WorkspaceInspectResult, WorktreeCreateRequest,
    WorktreeListRequest, WorktreeListResult, WorktreeRecord, WorktreeRemoveRequest,
};
use yolo_wire::error_code;
use yolo_wire::{ErrorBody, RequestEnvelope, ResponseEnvelope, method};

/// A failure the Host turns into a wire ErrorBody. It carries only stable
/// codes and messages; no raw git output becomes control authority.
#[derive(Debug)]
pub struct WorkspaceFailure {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

pub struct WorkspaceService {
    pub(crate) git: GitWorkspace,
    pub(crate) ledger: Option<WorktreeLedger>,
}

impl WorkspaceService {
    pub fn from_environment(home: &std::path::Path) -> Self {
        let ledger = WorktreeLedger::open(home.join(WORKTREE_LEDGER_FILE)).ok();
        let service = Self {
            git: GitWorkspace::from_environment(),
            ledger,
        };
        service.reconcile_ledger();
        service
    }

    #[cfg(test)]
    pub(crate) fn for_test(git: GitWorkspace, home: &std::path::Path) -> Self {
        let ledger = WorktreeLedger::open(home.join(WORKTREE_LEDGER_FILE)).ok();
        let service = Self { git, ledger };
        service.reconcile_ledger();
        service
    }

    pub fn inspect(
        &self,
        request: WorkspaceInspectRequest,
    ) -> Result<WorkspaceInspectResult, WorkspaceFailure> {
        if !request.root.starts_with('/') || request.root.contains('\0') {
            return Err(directory("workspace root must be an absolute path"));
        }
        let canonical = std::fs::canonicalize(&request.root)
            .map_err(|_| directory("workspace root is unavailable"))?;
        let metadata = std::fs::metadata(&canonical)
            .map_err(|_| directory("workspace root is unavailable"))?;
        if !metadata.is_dir() {
            return Err(directory("workspace root is not a directory"));
        }
        let git = match self.git.probe(&canonical) {
            Ok(version) => GitProbe::Available { version },
            Err(WorktreeError::Unavailable(reason)) => GitProbe::Unavailable { reason },
            Err(error) => GitProbe::Unavailable {
                reason: error.to_string(),
            },
        };
        Ok(WorkspaceInspectResult {
            canonical_root: canonical.display().to_string(),
            git,
        })
    }

    pub fn create_worktree(
        &self,
        request: WorktreeCreateRequest,
    ) -> Result<WorktreeRecord, WorkspaceFailure> {
        let record = self
            .create_with_ledger(&CreateWorktree {
                repository_root: std::path::PathBuf::from(&request.repository_root),
                worktree_id: request.worktree_id,
                branch: request.branch,
                base: request.base,
                trust_repository: request.trust_repository,
            })
            .map_err(failure)?;
        Ok(WorktreeRecord {
            path: record.path,
            branch: record.branch,
            head: record.head,
        })
    }

    pub fn remove_worktree(&self, request: WorktreeRemoveRequest) -> Result<(), WorkspaceFailure> {
        self.remove_with_ledger(
            std::path::Path::new(&request.repository_root),
            std::path::Path::new(&request.path),
        )
        .map_err(failure)
    }

    pub fn list_worktrees(
        &self,
        request: WorktreeListRequest,
    ) -> Result<WorktreeListResult, WorkspaceFailure> {
        let worktrees = self
            .git
            .list(std::path::Path::new(&request.repository_root))
            .map_err(failure)?
            .into_iter()
            .map(|record| WorktreeRecord {
                path: record.path,
                branch: record.branch,
                head: record.head,
            })
            .collect();
        Ok(WorktreeListResult { worktrees })
    }
}

fn directory(message: &str) -> WorkspaceFailure {
    WorkspaceFailure {
        code: error_code::INVALID_DIRECTORY,
        message: message.into(),
        retryable: false,
    }
}

fn failure(error: WorktreeError) -> WorkspaceFailure {
    let retryable = matches!(
        error,
        WorktreeError::Timeout | WorktreeError::Unknown(_) | WorktreeError::Io(_)
    );
    WorkspaceFailure {
        code: error.code(),
        message: error.to_string(),
        retryable,
    }
}

impl WorkspaceService {
    /// Dispatch one already-decoded wire request. The Host keeps a single arm
    /// for the whole workspace group.
    pub fn dispatch(&self, request: &RequestEnvelope) -> Result<ResponseEnvelope, ErrorBody> {
        match request.method.as_str() {
            method::WORKSPACE_INSPECT => respond(
                request,
                &self.inspect(decode(request)?).map_err(ErrorBody::from)?,
            ),
            method::WORKTREE_CREATE => respond(
                request,
                &self
                    .create_worktree(decode(request)?)
                    .map_err(ErrorBody::from)?,
            ),
            method::WORKTREE_REMOVE => {
                self.remove_worktree(decode(request)?)
                    .map_err(ErrorBody::from)?;
                Ok(ResponseEnvelope::empty(request))
            }
            method::WORKTREE_LIST => respond(
                request,
                &self
                    .list_worktrees(decode(request)?)
                    .map_err(ErrorBody::from)?,
            ),
            _ => Err(ErrorBody::new(
                error_code::UNSUPPORTED_METHOD,
                "unsupported workspace method",
                false,
            )),
        }
    }
}

impl From<WorkspaceFailure> for ErrorBody {
    fn from(failure: WorkspaceFailure) -> Self {
        Self::new(failure.code, failure.message, failure.retryable)
    }
}

fn respond<T: Serialize>(
    request: &RequestEnvelope,
    payload: &T,
) -> Result<ResponseEnvelope, ErrorBody> {
    ResponseEnvelope::success(request, payload)
        .map_err(|error| ErrorBody::invalid(error.to_string()))
}

fn decode<T: DeserializeOwned>(request: &RequestEnvelope) -> Result<T, ErrorBody> {
    request
        .decode_payload()
        .map_err(|error| ErrorBody::invalid(error.to_string()))
}
