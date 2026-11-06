use crate::FileService;
use std::{path::PathBuf, sync::atomic::AtomicBool};
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_helper_protocol::{
    Command, FILE_HELPER_SCHEMA, MAX_OPERATION_TIME, Payload, Request, RequestFields,
};
use yolo_hub_file_protocol::{
    FileRootAddress, FileRootConfiguration, FileRootSummary, FileRootSummaryFields, FileRootTarget,
    MAX_FILE_ROOTS,
};
use yolo_hub_identity::{FileRootEpoch, HubId};
use yolo_hub_terminal_protocol::PublicError;

impl FileService {
    pub async fn prepare(
        origin: HubId,
        roots: &[FileRootConfiguration],
        protected: Vec<PathBuf>,
        program: Option<PathBuf>,
    ) -> Result<Self, PublicError> {
        if roots.len() > MAX_FILE_ROOTS {
            return Err(PublicError::InvalidRequest);
        }
        let mut service = Self {
            origin,
            epoch: FileRootEpoch::new(),
            specification: roots.to_vec(),
            roots: Default::default(),
            protected: Vec::new(),
            executor: None,
            closed: AtomicBool::new(false),
        };
        if roots.is_empty() {
            return Ok(service);
        }
        let executor = FileExecutor::new(program.ok_or(PublicError::Unsupported)?)?;
        let roots = roots
            .iter()
            .map(|root| {
                FileRootSummary::new(FileRootSummaryFields {
                    target: FileRootTarget {
                        address: FileRootAddress {
                            hub_id: origin,
                            root_id: root.fields().root_id,
                        },
                        epoch: service.epoch,
                    },
                    name: root.fields().name.clone(),
                    path: root.fields().path.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        service.protected = protected
            .iter()
            .map(|path| {
                path.to_str()
                    .map(str::to_owned)
                    .ok_or(PublicError::InvalidMetadata)
            })
            .collect::<Result<_, _>>()?;
        let request = Request::new(RequestFields {
            schema_version: FILE_HELPER_SCHEMA,
            request_id: uuid::Uuid::new_v4(),
            origin,
            protected: service.protected.clone(),
            command: Command::Inspect { roots },
        })?;
        let result = executor
            .execute(request, tokio::time::Instant::now() + MAX_OPERATION_TIME)
            .await?;
        let bindings = match result {
            Payload::Inspected { bindings } => bindings,
            Payload::Error { code } => return Err(code),
            _ => return Err(PublicError::Unreachable),
        };
        service.roots = bindings
            .into_iter()
            .map(|binding| (binding.root.fields().target.address.root_id, binding))
            .collect();
        service.executor = Some(executor);
        Ok(service)
    }
}
