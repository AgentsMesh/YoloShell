#[cfg(target_os = "linux")]
include!("startup_containment.rs");
include!("runtime_preparation.rs");

pub(crate) const RUNTIME_FAILURE_BEFORE_CLEANUP_LOG: &str =
    "session worker runtime failed before cleanup settlement";

pub async fn run(paths: Paths, terminal_id: Uuid) -> Result<()> {
    let provider_runtime = Arc::new(yolo_agent_providers::builtin_provider_runtime());
    let mut cleanup_scope = StartupCleanupScope::new()?;
    let result = run_session(paths, terminal_id, provider_runtime, &mut cleanup_scope).await;
    if let Err(error) = &result {
        tracing::error!(
            %terminal_id,
            error = format_args!("{error:#}"),
            "{RUNTIME_FAILURE_BEFORE_CLEANUP_LOG}"
        );
    }
    #[cfg(test)]
    runtime_result_test_hook::record(terminal_id, &result);
    settle_runtime_result(result, cleanup_scope).await
}
