use super::StartupCleanupScope;
use anyhow::Result;

pub(crate) async fn settle(result: Result<()>, cleanup_scope: StartupCleanupScope) -> Result<()> {
    match cleanup_scope.settle().await {
        Ok(()) => result,
        Err(cleanup) => Err(match result {
            Ok(()) => cleanup,
            Err(runtime) => cleanup.context(runtime.to_string()),
        }),
    }
}
