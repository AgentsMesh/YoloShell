use super::*;

const REGISTRY_ENTRY_OCCUPIED_DETAIL: &str =
    "exact PTY secondary-custody registry entry is already occupied";
const REGISTRY_ENTRY_MISSING_DETAIL: &str =
    "exact PTY secondary-custody active registry entry is unavailable";
const REGISTRY_ENTRY_MISMATCH_DETAIL: &str =
    "exact PTY secondary-custody active registry entry changed identity";
const REGISTRY_RESERVE_DETAIL: &str =
    "reserve exact PTY secondary-custody registry retirement capacity";

pub(super) struct ExactRegistryRetirement {
    key: SecondaryCustodyKey,
    ordinal: retirement::SecondaryCustodySuccessionOrdinal,
}

impl SecondaryCustodyRegistry {
    pub(super) async fn lock(&self, terminal_id: Uuid) -> tokio::sync::OwnedMutexGuard<()> {
        self.entries.gates.lock(terminal_id).await
    }

    pub(super) fn current(
        &self,
        runtime: &CustodyRuntimeIdentity,
    ) -> Option<Arc<SecondaryCustodyLease>> {
        super::super::lock_recover(&self.entries.values)
            .active
            .get(&SecondaryCustodyKey::from_runtime(runtime))
            .cloned()
    }

    pub(super) fn admission(&self, runtime: &CustodyRuntimeIdentity) -> SecondaryCustodyAdmission {
        let values = super::super::lock_recover(&self.entries.values);
        let key = SecondaryCustodyKey::from_runtime(runtime);
        if let Some(retirement) = values.retired.get(&key) {
            return SecondaryCustodyAdmission::Retired {
                runtime: retirement.runtime().clone(),
                state: retirement.state(),
            };
        }
        let Some(lease) = values.active.get(&key).cloned() else {
            return SecondaryCustodyAdmission::Absent;
        };
        let (state, ready) = lease.admission_snapshot();
        SecondaryCustodyAdmission::Active {
            lease,
            state,
            ready,
        }
    }

    pub(in crate::host) fn retired_for_settlement(&self, runtime: &CustodyRuntimeIdentity) -> bool {
        let values = super::super::lock_recover(&self.entries.values);
        values
            .retired
            .get(&SecondaryCustodyKey::from_runtime(runtime))
            .is_some_and(|retirement| {
                retirement.matches_runtime(runtime)
                    && matches!(
                        retirement.state(),
                        SecondaryCustodyState::ReleaseAckPending
                            | SecondaryCustodyState::Released
                            | SecondaryCustodyState::ReleaseUncertain
                            | SecondaryCustodyState::Superseded
                    )
            })
    }

    pub(super) fn install(&self, lease: Arc<SecondaryCustodyLease>) -> Result<(), WorkerRpcError> {
        let key = SecondaryCustodyKey::from_runtime(&lease.runtime);
        let mut values = super::super::lock_recover(&self.entries.values);
        if values.active.contains_key(&key) || values.retired.contains_key(&key) {
            return Err(WorkerRpcError::protocol(REGISTRY_ENTRY_OCCUPIED_DETAIL));
        }
        values.active.try_reserve(1).map_err(|error| {
            let detail = super::containment::owned_display_detail(error);
            WorkerRpcError::transport(format!("{REGISTRY_RESERVE_DETAIL}: {detail}"))
        })?;
        values.active_succession.try_reserve(1).map_err(|error| {
            let detail = super::containment::owned_display_detail(error);
            WorkerRpcError::transport(format!("{REGISTRY_RESERVE_DETAIL}: {detail}"))
        })?;
        let succession_ordinal = values.succession.allocate()?;
        lease.bind_registry(Arc::downgrade(&self.entries));
        assert!(
            values
                .active_succession
                .insert(key, succession_ordinal)
                .is_none(),
            "{REGISTRY_ENTRY_MISMATCH_DETAIL}"
        );
        values.active.insert(key, lease);
        Ok(())
    }

    pub(super) fn predecessors(
        &self,
        runtime: &CustodyRuntimeIdentity,
    ) -> Vec<Arc<SecondaryCustodyLease>> {
        let successor = SecondaryCustodyKey::from_runtime(runtime);
        let values = super::super::lock_recover(&self.entries.values);
        let active = values
            .active
            .iter()
            .filter(|(key, _)| key.terminal_id == runtime.terminal_id && **key != successor)
            .map(|(_, lease)| Arc::clone(lease));
        let retiring = values.retired.iter().filter_map(|(key, retirement)| {
            (key.terminal_id == runtime.terminal_id && *key != successor)
                .then(|| retirement.retained_predecessor())
                .flatten()
        });
        active.chain(retiring).collect()
    }

    pub(in crate::host) fn ready_for(
        &self,
        record: &RuntimeRecord,
        holder: CustodyHolderIdentity,
        generation: u64,
    ) -> bool {
        self.current(&CustodyRuntimeIdentity::from_record(record))
            .is_some_and(|lease| lease.matches(record, holder, generation) && lease.is_ready())
    }
}

pub(super) fn prepare_exact_retirement(
    values: &mut SecondaryCustodyValues,
    lease: &Arc<SecondaryCustodyLease>,
) -> Result<ExactRegistryRetirement, WorkerRpcError> {
    let key = SecondaryCustodyKey::from_runtime(&lease.runtime);
    let active = values
        .active
        .get(&key)
        .ok_or_else(|| WorkerRpcError::protocol(REGISTRY_ENTRY_MISSING_DETAIL))?;
    if !Arc::ptr_eq(active, lease) || values.retired.contains_key(&key) {
        return Err(WorkerRpcError::protocol(REGISTRY_ENTRY_MISMATCH_DETAIL));
    }
    let ordinal = *values
        .active_succession
        .get(&key)
        .ok_or_else(|| WorkerRpcError::protocol(REGISTRY_ENTRY_MISMATCH_DETAIL))?;
    values.retired.try_reserve(1).map_err(|error| {
        let detail = super::containment::owned_display_detail(error);
        WorkerRpcError::transport(format!("{REGISTRY_RESERVE_DETAIL}: {detail}"))
    })?;
    Ok(ExactRegistryRetirement { key, ordinal })
}

impl ExactRegistryRetirement {
    pub(super) fn commit_retained(
        self,
        values: &mut SecondaryCustodyValues,
        lease: &Arc<SecondaryCustodyLease>,
    ) {
        let active = self.take_active(values, lease);
        let retirement = retirement::SecondaryCustodyRetirement::retained(active, self.ordinal);
        let replaced = values.retired.insert(self.key, retirement);
        assert!(replaced.is_none(), "{REGISTRY_ENTRY_MISMATCH_DETAIL}");
    }

    pub(super) fn commit_terminal(
        self,
        values: &mut SecondaryCustodyValues,
        lease: &Arc<SecondaryCustodyLease>,
        state: SecondaryCustodyState,
    ) {
        let active = self.take_active(values, lease);
        let retirement =
            retirement::SecondaryCustodyRetirement::terminal(&active, state, self.ordinal);
        let replaced = values.retired.insert(self.key, retirement);
        assert!(replaced.is_none(), "{REGISTRY_ENTRY_MISMATCH_DETAIL}");
        retirement::compact_terminal_retirements(values, self.key.terminal_id);
    }

    fn take_active(
        &self,
        values: &mut SecondaryCustodyValues,
        lease: &Arc<SecondaryCustodyLease>,
    ) -> Arc<SecondaryCustodyLease> {
        let active = values
            .active
            .remove(&self.key)
            .expect(REGISTRY_ENTRY_MISSING_DETAIL);
        assert!(
            Arc::ptr_eq(&active, lease),
            "{REGISTRY_ENTRY_MISMATCH_DETAIL}"
        );
        let ordinal = values
            .active_succession
            .remove(&self.key)
            .expect(REGISTRY_ENTRY_MISSING_DETAIL);
        assert_eq!(ordinal, self.ordinal, "{REGISTRY_ENTRY_MISMATCH_DETAIL}");
        active
    }
}
