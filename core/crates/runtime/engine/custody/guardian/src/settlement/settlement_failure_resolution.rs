use super::ProgressFailureLog;
use crate::namespace_lease::{NamespaceLease, NamespaceVerification};
use crate::pty_custody::CustodyRuntimeIdentity;

pub(super) const NAMESPACE_LOST_CONTEXT: &str =
    "detached guardian durable namespace ownership was lost";
pub(super) const ORPHANED_RUNTIME_CONTEXT: &str =
    "detached guardian durable progress failed after exact runtime exit";
pub(super) const INDETERMINATE_NAMESPACE_CONTEXT: &str =
    "detached guardian durable namespace verification was indeterminate";

pub(in crate::custody_guardian) enum Resolution {
    Retry,
    RetainUntilSettled(anyhow::Error),
}

pub(in crate::custody_guardian) fn resolve(
    namespace: &NamespaceLease,
    runtime: &CustodyRuntimeIdentity,
    error: anyhow::Error,
    log: &mut ProgressFailureLog,
) -> Resolution {
    resolve_verification(namespace.verification(), runtime, error, log)
}

pub(super) fn resolve_verification(
    verification: NamespaceVerification,
    runtime: &CustodyRuntimeIdentity,
    error: anyhow::Error,
    log: &mut ProgressFailureLog,
) -> Resolution {
    match verification {
        NamespaceVerification::IdentityLost {
            error: namespace_error,
            ..
        } => {
            let namespace_detail = super::super::terminal_reporting::error_detail(namespace_error);
            Resolution::RetainUntilSettled(
                error.context(format!("{NAMESPACE_LOST_CONTEXT}: {namespace_detail}")),
            )
        }
        NamespaceVerification::Verified => resolve_retained_runtime(runtime, error, None, log),
        NamespaceVerification::Indeterminate(namespace_error) => {
            resolve_retained_runtime(runtime, error, Some(namespace_error), log)
        }
    }
}

fn resolve_retained_runtime(
    runtime: &CustodyRuntimeIdentity,
    error: anyhow::Error,
    namespace_error: Option<anyhow::Error>,
    log: &mut ProgressFailureLog,
) -> Resolution {
    match super::runtime_processes_gone(runtime) {
        Ok(true) => Resolution::RetainUntilSettled(orphaned_error(error, namespace_error)),
        Ok(false) => {
            log.report(error, namespace_error);
            Resolution::Retry
        }
        Err(observation_error) => {
            let secondary = match namespace_error {
                Some(namespace_error) => namespace_error.context(format!(
                    "{}: {observation_error:#}",
                    super::super::settlement::PROCESS_DETAIL
                )),
                None => observation_error,
            };
            log.report(error, Some(secondary));
            Resolution::Retry
        }
    }
}

fn orphaned_error(error: anyhow::Error, namespace_error: Option<anyhow::Error>) -> anyhow::Error {
    let Some(namespace_error) = namespace_error else {
        return error.context(ORPHANED_RUNTIME_CONTEXT);
    };
    let detail = super::super::terminal_reporting::error_detail(namespace_error);
    error.context(format!(
        "{ORPHANED_RUNTIME_CONTEXT}: {INDETERMINATE_NAMESPACE_CONTEXT}: {detail}"
    ))
}
