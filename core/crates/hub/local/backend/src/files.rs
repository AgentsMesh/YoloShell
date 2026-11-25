use crate::LocalBackend;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use yolo_hub_file_protocol::{FileRootConfiguration, ReadFileRequest};
use yolo_hub_file_service::FileService;
use yolo_hub_identity::FileRootId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    pub fn with_file_helper(mut self, program: PathBuf) -> Result<Self, PublicError> {
        if !program.is_absolute() || program.file_name().is_none() {
            return Err(PublicError::InvalidMetadata);
        }
        self.file_helper = Some(program);
        Ok(self)
    }
    pub async fn prepare_files(
        &self,
        hub_state: &Path,
        roots: &[FileRootConfiguration],
    ) -> Result<Self, PublicError> {
        let files = FileService::prepare(
            self.hub,
            roots,
            vec![hub_state.to_path_buf(), self.paths.home().to_path_buf()],
            self.file_helper.clone(),
        )
        .await?;
        let mut backend = self.clone();
        backend.files = Some(Arc::new(files));
        Ok(backend)
    }
    pub fn files_match(&self, roots: &[FileRootConfiguration]) -> bool {
        self.files
            .as_ref()
            .map_or_else(|| roots.is_empty(), |files| files.configured_as(roots))
    }
    pub fn close_files(&self) {
        if let Some(files) = &self.files {
            files.close();
        }
    }
    pub(crate) fn list_file_roots(&self, after: Option<FileRootId>) -> Result<Reply, PublicError> {
        let (epoch, roots, next) = match &self.files {
            Some(files) => files.list(after)?,
            None => (self.file_epoch, Vec::new(), None),
        };
        Ok(Reply::FileRoots { epoch, roots, next })
    }
    pub(crate) async fn read_file(&self, request: ReadFileRequest) -> Result<Reply, PublicError> {
        let files = self.files.as_ref().ok_or(PublicError::NotFound)?;
        Ok(Reply::File {
            file: files.read(request).await?,
        })
    }
}
