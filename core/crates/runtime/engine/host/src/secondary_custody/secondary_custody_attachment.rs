use super::*;

#[derive(Debug)]
enum DiagnosticLocation {
    Absent,
    Active,
    Retired,
}

impl SecondaryCustodyRegistry {
    pub(in crate::host) fn diagnostic_state(&self, runtime: &CustodyRuntimeIdentity) -> String {
        let (location, state, matches_runtime) = match self.admission(runtime) {
            SecondaryCustodyAdmission::Active { lease, state, .. } => (
                DiagnosticLocation::Active,
                Some(state),
                lease.runtime == *runtime,
            ),
            SecondaryCustodyAdmission::Retired {
                runtime: current,
                state,
            } => (
                DiagnosticLocation::Retired,
                Some(state),
                current == *runtime,
            ),
            SecondaryCustodyAdmission::Absent => (DiagnosticLocation::Absent, None, false),
        };
        format!("location={location:?},state={state:?},matches_runtime={matches_runtime}")
    }

    pub(in crate::host) fn released_for_read_only_attachment(
        &self,
        runtime: &CustodyRuntimeIdentity,
    ) -> bool {
        self.exact_runtime_is_released(runtime)
    }

    pub(in crate::host) fn released_for_terminal_worker_shutdown(
        &self,
        runtime: &CustodyRuntimeIdentity,
    ) -> bool {
        self.exact_runtime_is_released(runtime)
    }

    fn exact_runtime_is_released(&self, runtime: &CustodyRuntimeIdentity) -> bool {
        super::super::lock_recover(&self.entries.values)
            .retired
            .get(&SecondaryCustodyKey::from_runtime(runtime))
            .is_some_and(|retirement| {
                retirement.matches_runtime(runtime)
                    && retirement.state() == SecondaryCustodyState::Released
            })
    }
}
