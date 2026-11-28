use super::*;

const TERMINAL_RETIREMENT_STATE_DETAIL: &str =
    "PTY secondary-custody terminal retirement has a nonterminal state";
const SUCCESSION_ORDINAL_EXHAUSTED_DETAIL: &str =
    "PTY secondary-custody succession ordinal is exhausted";
const SUCCESSION_ORDINAL_STEP: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SecondaryCustodySuccessionOrdinal(u64);

#[derive(Default)]
pub(super) struct SecondaryCustodySuccession {
    latest: u64,
}

pub(super) enum SecondaryCustodyRetirement {
    Retained {
        lease: Arc<SecondaryCustodyLease>,
        ordinal: SecondaryCustodySuccessionOrdinal,
    },
    Terminal(SecondaryCustodyRetirementTombstone),
}

pub(super) struct SecondaryCustodyRetirementTombstone {
    runtime: CustodyRuntimeIdentity,
    state: SecondaryCustodyState,
    ordinal: SecondaryCustodySuccessionOrdinal,
}

impl SecondaryCustodySuccession {
    pub(super) fn allocate(&mut self) -> Result<SecondaryCustodySuccessionOrdinal, WorkerRpcError> {
        self.latest = self
            .latest
            .checked_add(SUCCESSION_ORDINAL_STEP)
            .ok_or_else(|| WorkerRpcError::protocol(SUCCESSION_ORDINAL_EXHAUSTED_DETAIL))?;
        Ok(SecondaryCustodySuccessionOrdinal(self.latest))
    }
}

impl SecondaryCustodyRetirement {
    pub(super) fn retained(
        lease: Arc<SecondaryCustodyLease>,
        ordinal: SecondaryCustodySuccessionOrdinal,
    ) -> Self {
        Self::Retained { lease, ordinal }
    }

    pub(super) fn terminal(
        lease: &SecondaryCustodyLease,
        state: SecondaryCustodyState,
        ordinal: SecondaryCustodySuccessionOrdinal,
    ) -> Self {
        assert!(
            matches!(
                state,
                SecondaryCustodyState::Released | SecondaryCustodyState::Superseded
            ),
            "{TERMINAL_RETIREMENT_STATE_DETAIL}"
        );
        Self::Terminal(SecondaryCustodyRetirementTombstone {
            runtime: lease.runtime.clone(),
            state,
            ordinal,
        })
    }

    pub(super) fn replace_with_terminal(
        &mut self,
        lease: &SecondaryCustodyLease,
        state: SecondaryCustodyState,
    ) {
        let ordinal = self.succession_ordinal();
        *self = Self::terminal(lease, state, ordinal);
    }

    pub(super) fn state(&self) -> SecondaryCustodyState {
        match self {
            Self::Retained { lease, .. } => lease.state(),
            Self::Terminal(tombstone) => tombstone.state,
        }
    }

    pub(super) fn runtime(&self) -> &CustodyRuntimeIdentity {
        match self {
            Self::Retained { lease, .. } => &lease.runtime,
            Self::Terminal(tombstone) => &tombstone.runtime,
        }
    }

    pub(super) fn matches_runtime(&self, runtime: &CustodyRuntimeIdentity) -> bool {
        self.runtime() == runtime
    }

    pub(super) fn retained_predecessor(&self) -> Option<Arc<SecondaryCustodyLease>> {
        match self {
            Self::Retained { lease, .. }
                if !matches!(
                    lease.state(),
                    SecondaryCustodyState::Released | SecondaryCustodyState::Superseded
                ) =>
            {
                Some(Arc::clone(lease))
            }
            Self::Retained { .. } | Self::Terminal(_) => None,
        }
    }

    pub(super) fn references(&self, lease: &Arc<SecondaryCustodyLease>) -> bool {
        matches!(self, Self::Retained { lease: current, .. } if Arc::ptr_eq(current, lease))
    }

    pub(super) fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }

    fn succession_ordinal(&self) -> SecondaryCustodySuccessionOrdinal {
        match self {
            Self::Retained { ordinal, .. } => *ordinal,
            Self::Terminal(tombstone) => tombstone.ordinal,
        }
    }

    #[cfg(test)]
    pub(super) fn retains_lease(&self) -> bool {
        matches!(self, Self::Retained { .. })
    }
}

pub(super) fn compact_terminal_retirements(values: &mut SecondaryCustodyValues, terminal_id: Uuid) {
    let newest = values
        .retired
        .iter()
        .filter(|(key, retirement)| key.terminal_id == terminal_id && retirement.is_terminal())
        .max_by_key(|(_, retirement)| retirement.succession_ordinal())
        .map(|(key, _)| *key);
    values.retired.retain(|key, retirement| {
        key.terminal_id != terminal_id || !retirement.is_terminal() || Some(*key) == newest
    });
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/secondary_custody/secondary_custody_retirement_inline_tests.rs"]
mod tests;
