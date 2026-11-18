use crate::host::{WorkspaceInspectRequest, WorkspaceInspectResult};
use crate::{Client, ClientError, protocol::method};
use yolo_runtime_protocol::{
    WorktreeCreateRequest, WorktreeListRequest, WorktreeListResult, WorktreeRecord,
    WorktreeRemoveRequest,
};

impl Client {
    pub async fn workspace_inspect(
        &self,
        request: &WorkspaceInspectRequest,
    ) -> Result<WorkspaceInspectResult, ClientError> {
        self.request(method::WORKSPACE_INSPECT, request).await
    }
}

impl Client {
    pub async fn worktree_create(
        &self,
        request: &WorktreeCreateRequest,
    ) -> Result<WorktreeRecord, ClientError> {
        self.request(method::WORKTREE_CREATE, request).await
    }

    pub async fn worktree_remove(
        &self,
        request: &WorktreeRemoveRequest,
    ) -> Result<(), ClientError> {
        self.request::<_, ()>(method::WORKTREE_REMOVE, request)
            .await
    }

    pub async fn worktree_list(
        &self,
        request: &WorktreeListRequest,
    ) -> Result<WorktreeListResult, ClientError> {
        self.request(method::WORKTREE_LIST, request).await
    }
}
