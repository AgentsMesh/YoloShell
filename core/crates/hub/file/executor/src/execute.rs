use crate::{constants::MAX_CONCURRENT_FILE_HELPERS, launch};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    sync::{Semaphore, watch},
    time::Instant,
};
use yolo_hub_file_helper_protocol::{MAX_OPERATION_TIME, Payload, Request};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone)]
pub struct FileExecutor {
    program: PathBuf,
    permits: Arc<Semaphore>,
    closed: watch::Sender<bool>,
}

impl FileExecutor {
    pub fn new(program: PathBuf) -> Result<Self, PublicError> {
        if !program.is_absolute() || program.file_name().is_none() {
            return Err(PublicError::InvalidMetadata);
        }
        let (closed, _) = watch::channel(false);
        Ok(Self {
            program,
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_FILE_HELPERS)),
            closed,
        })
    }
    pub fn close(&self) {
        self.closed.send_replace(true);
        self.permits.close();
    }
    pub async fn execute(
        &self,
        request: Request,
        deadline: Instant,
    ) -> Result<Payload, PublicError> {
        let deadline = deadline.min(Instant::now() + MAX_OPERATION_TIME);
        let mut closed = self.closed.subscribe();
        if *closed.borrow() || Instant::now() >= deadline {
            return Err(PublicError::Unreachable);
        }
        let operation = async {
            let _permit = self
                .permits
                .acquire()
                .await
                .map_err(|_| PublicError::Unreachable)?;
            if *self.closed.borrow() {
                return Err(PublicError::Unreachable);
            }
            launch::call(&self.program, &request, deadline).await
        };
        tokio::select! {
            biased;
            _ = closed.changed() => Err(PublicError::Unreachable),
            _ = tokio::time::sleep_until(deadline) => Err(PublicError::Unreachable),
            result = operation => {
                if *self.closed.borrow() { Err(PublicError::Unreachable) } else { result }
            }
        }
    }
}
