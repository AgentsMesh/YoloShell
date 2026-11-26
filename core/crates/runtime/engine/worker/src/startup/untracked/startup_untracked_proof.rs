use super::UnreleasedBootstrapGate;

pub(crate) struct UntrackedStartupProducersGone {
    pub(crate) _gate: UnreleasedBootstrapGate,
}

impl UntrackedStartupProducersGone {
    pub(crate) fn from_exited_closed_gate(gate: UnreleasedBootstrapGate) -> Self {
        Self { _gate: gate }
    }
}
