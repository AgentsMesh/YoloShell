use super::*;

pub(crate) const PROVIDER_PROCESS_SCAN_FAILED_LOG: &str = "provider resume process scan was inconclusive";
pub(crate) const PROVIDER_PROCESS_SCAN_JOIN_FAILED_LOG: &str =
    "provider resume process scan task did not complete";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderRuntimeMembership {
    NotObserved,
    Contradicted,
    Verified(i64),
}

impl Host {
    pub(crate) async fn provider_runtime_membership(
        &self,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
        binding: &AgentBinding,
        deadline: HostOperationDeadline,
    ) -> ProviderRuntimeMembership {
        let root = crate::process::ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        let Some(scan) = self
            .provider_process_scans
            .observe(self.provider_process_scanner.clone(), root, deadline)
            .await
        else {
            return ProviderRuntimeMembership::NotObserved;
        };
        let scan = match scan.as_ref() {
            ProviderProcessScanOutcome::Observed(scan) => scan,
            ProviderProcessScanOutcome::Failed(error) => {
                tracing::warn!(%error, PROVIDER_PROCESS_SCAN_FAILED_LOG);
                return ProviderRuntimeMembership::NotObserved;
            }
            ProviderProcessScanOutcome::JoinFailed(error) => {
                tracing::error!(%error, PROVIDER_PROCESS_SCAN_JOIN_FAILED_LOG);
                return ProviderRuntimeMembership::NotObserved;
            }
        };
        if scan.started_at_ms <= 0
            || scan.completed_at_ms < scan.started_at_ms
            || scan.started_at_ms <= binding.updated_at_ms
        {
            return ProviderRuntimeMembership::NotObserved;
        }
        let crate::agent::FencedDescendants::Stable(processes) = &scan.descendants else {
            return ProviderRuntimeMembership::NotObserved;
        };
        provider_membership_from_processes(
            self.agent_registry.as_ref(),
            transaction,
            runtime,
            binding,
            processes,
            scan.completed_at_ms,
        )
    }
}

pub(crate) fn provider_membership_from_processes(
    registry: &crate::agent::AgentRegistry,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
    binding: &AgentBinding,
    processes: &[crate::agent::ObservedProcess],
    observed_at_ms: i64,
) -> ProviderRuntimeMembership {
    let ResumeTarget::Provider {
        agent_kind,
        provider_session_id,
        ..
    } = &transaction.target
    else {
        return ProviderRuntimeMembership::Contradicted;
    };
    let Some(identity) = binding.process_pid.zip(binding.process_start_time) else {
        return ProviderRuntimeMembership::Contradicted;
    };
    let Some(process) = processes
        .iter()
        .find(|process| process.identity_matches(identity.0, identity.1))
    else {
        return ProviderRuntimeMembership::NotObserved;
    };
    let Some(environment) = process.environment.as_ref() else {
        return ProviderRuntimeMembership::NotObserved;
    };
    if process.argv.is_empty() {
        return ProviderRuntimeMembership::NotObserved;
    }
    let Some(adapter) = registry.adapter(agent_kind) else {
        return ProviderRuntimeMembership::Contradicted;
    };
    if !process.is_foreground()
        || !adapter.matches_process(process)
        || adapter.process_resume_session_id(process).as_deref() != Some(provider_session_id.as_str())
    {
        return ProviderRuntimeMembership::Contradicted;
    }
    let session_matches = canonical_environment_uuid_matches(
        environment,
        crate::agent::constants::YOLO_TERMINAL_ID_ENV,
        runtime.terminal_id,
    );
    let epoch_matches = canonical_environment_uuid_matches(
        environment,
        crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV,
        runtime.runtime_epoch,
    );
    if session_matches && epoch_matches {
        ProviderRuntimeMembership::Verified(observed_at_ms)
    } else {
        ProviderRuntimeMembership::Contradicted
    }
}

pub(crate) fn canonical_environment_uuid_matches(
    environment: &std::collections::BTreeMap<String, String>,
    key: &str,
    expected: Uuid,
) -> bool {
    let canonical = expected.hyphenated().to_string();
    environment
        .get(key)
        .is_some_and(|value| value == &canonical && Uuid::parse_str(value).ok() == Some(expected))
}
