use super::router::{
    CodexRecoveredRoute, CodexRouteEntry, CodexRouterError, CodexSessionRouter,
    DurableCodexAdmission,
};
use super::runtime::CodexHookRuntime;
use crate::codex_hooks::{CodexAdmissionAuthority, CodexAdmissionRecord};
use anyhow::{Context, Result};
use std::sync::Arc;

pub(super) fn install<Builder, Validate>(
    router: &CodexSessionRouter,
    admission: &CodexAdmissionAuthority,
    builder: Builder,
    validate: Validate,
) -> Result<CodexRecoveredRoute>
where
    Builder: FnOnce(&CodexAdmissionRecord) -> Result<Arc<CodexHookRuntime>>,
    Validate: FnOnce(&CodexRouteEntry) -> Result<()>,
{
    let durable =
        DurableCodexAdmission::from_authority(admission.clone()).map_err(anyhow::Error::new)?;
    router
        .recover_committed(
            durable,
            |record| builder(record).map_err(|_| CodexRouterError::Runtime),
            |entry| validate(entry).map_err(|_| CodexRouterError::Admission),
        )
        .map_err(|error: CodexRouterError| anyhow::Error::new(error))
        .context(super::constants::CODEX_ROUTE_RECOVERY_ERROR)
}
