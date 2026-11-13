use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_helper_protocol::{
    Command, FILE_HELPER_SCHEMA, MAX_OPERATION_TIME, Payload, Request, RequestFields, RootBinding,
};
use yolo_hub_file_protocol::{
    FileRootConfiguration, FileRootSummary, MAX_FILE_ROOT_PAGE, ReadFileRequest, ReadFileResult,
};
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};
use yolo_hub_terminal_protocol::PublicError;

pub struct FileService {
    pub(crate) origin: HubId,
    pub(crate) epoch: FileRootEpoch,
    pub(crate) specification: Vec<FileRootConfiguration>,
    pub(crate) roots: BTreeMap<FileRootId, RootBinding>,
    pub(crate) protected: Vec<String>,
    pub(crate) executor: Option<FileExecutor>,
    pub(crate) closed: AtomicBool,
}

impl FileService {
    pub fn configured_as(&self, roots: &[FileRootConfiguration]) -> bool {
        !self.closed.load(Ordering::Acquire) && self.specification == roots
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        if let Some(executor) = &self.executor {
            executor.close();
        }
    }
    pub fn list(
        &self,
        after: Option<FileRootId>,
    ) -> Result<(FileRootEpoch, Vec<FileRootSummary>, Option<FileRootId>), PublicError> {
        self.require_open()?;
        let mut roots = self
            .roots
            .iter()
            .filter(|(id, _)| after.is_none_or(|after| **id > after))
            .take(MAX_FILE_ROOT_PAGE + 1)
            .map(|(_, binding)| binding.root.clone())
            .collect::<Vec<_>>();
        let more = roots.len() > MAX_FILE_ROOT_PAGE;
        roots.truncate(MAX_FILE_ROOT_PAGE);
        let next = more
            .then(|| {
                roots
                    .last()
                    .map(|root| root.fields().target.address.root_id)
            })
            .flatten();
        Ok((self.epoch, roots, next))
    }
    pub async fn read(&self, request: ReadFileRequest) -> Result<ReadFileResult, PublicError> {
        self.require_open()?;
        let target = request.fields().target;
        if target.address.hub_id != self.origin {
            return Err(PublicError::OriginMismatch);
        }
        let binding = self
            .roots
            .get(&target.address.root_id)
            .ok_or(PublicError::NotFound)?;
        if binding.root.fields().target != target {
            return Err(PublicError::TargetMismatch);
        }
        let executor = self.executor.as_ref().ok_or(PublicError::Unreachable)?;
        let operation = Request::new(RequestFields {
            schema_version: FILE_HELPER_SCHEMA,
            request_id: uuid::Uuid::new_v4(),
            origin: self.origin,
            protected: self.protected.clone(),
            command: Command::Read {
                binding: binding.clone(),
                request,
            },
        })?;
        let result = executor
            .execute(operation, tokio::time::Instant::now() + MAX_OPERATION_TIME)
            .await?;
        self.require_open()?;
        match result {
            Payload::Read { file } => Ok(file),
            Payload::Error { code } => Err(code),
            _ => Err(PublicError::Unreachable),
        }
    }
    fn require_open(&self) -> Result<(), PublicError> {
        if self.closed.load(Ordering::Acquire) {
            Err(PublicError::Unreachable)
        } else {
            Ok(())
        }
    }
}

impl Drop for FileService {
    fn drop(&mut self) {
        self.close();
    }
}
