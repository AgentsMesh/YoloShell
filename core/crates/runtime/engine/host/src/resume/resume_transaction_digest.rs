use super::*;

#[derive(Serialize)]
pub(crate) struct ResumeTransactionDigestBody {
    schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    previous_runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    planned_runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    worker_id: Uuid,
    intent: ResumeIntent,
    target: ResumeTarget,
    launch: WorkerLaunchSpec,
    launch_digest: String,
    created_at_ms: i64,
}

impl From<&ResumeTransaction> for ResumeTransactionDigestBody {
    fn from(transaction: &ResumeTransaction) -> Self {
        Self {
            schema_version: transaction.schema_version,
            terminal_id: transaction.terminal_id,
            previous_runtime_epoch: transaction.previous_runtime_epoch,
            planned_runtime_epoch: transaction.planned_runtime_epoch,
            worker_id: transaction.worker_id,
            intent: transaction.intent,
            target: transaction.target.clone(),
            launch: transaction.launch.clone(),
            launch_digest: transaction.launch_digest.clone(),
            created_at_ms: transaction.created_at_ms,
        }
    }
}

pub(crate) fn resume_transaction_digest(transaction: &ResumeTransaction) -> Result<String> {
    let body = ResumeTransactionDigestBody::from(transaction);
    let bytes = serde_json::to_vec(&body).context(RESUME_TRANSACTION_SERIALIZE_CONTEXT)?;
    Ok(yolo_crypto::sha256_hex(&bytes))
}

pub(crate) fn resume_launch_digest(launch: &WorkerLaunchSpec) -> Result<String> {
    let bytes = serde_json::to_vec(launch).context(RESUME_LAUNCH_SERIALIZE_CONTEXT)?;
    Ok(yolo_crypto::sha256_hex(&bytes))
}
