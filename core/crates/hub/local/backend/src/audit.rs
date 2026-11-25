use crate::LocalBackend;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    pub(crate) fn export_audit(
        &self,
        after: Option<u64>,
        limit: u16,
    ) -> Result<Reply, PublicError> {
        let path = self.audit.as_deref().ok_or(PublicError::Unsupported)?;
        let lines = yolo_hub_audit::read_export(path, after, usize::from(limit))
            .map_err(|_| PublicError::Internal)?;
        Ok(Reply::Audit { lines })
    }
}
