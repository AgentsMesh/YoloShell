use super::super::runtime::IntakeReplayReport;

const ROUTE_INCREMENT: usize = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::worker::codex_hook) struct IntakeRoundReport {
    pub(in crate::worker::codex_hook) routes: usize,
    pub(in crate::worker::codex_hook) failed_routes: usize,
    pub(in crate::worker::codex_hook) applied: usize,
    pub(in crate::worker::codex_hook) replayed: usize,
    pub(in crate::worker::codex_hook) stale: usize,
    pub(in crate::worker::codex_hook) quarantined: usize,
    pub(in crate::worker::codex_hook) deferred: usize,
    pub(in crate::worker::codex_hook) retirement_pending: usize,
    pub(in crate::worker::codex_hook) retired: usize,
    pub(in crate::worker::codex_hook) retirement_missing: usize,
}

pub(super) enum IntakeRouteReport {
    Active(IntakeReplayReport),
    RetirementPending(IntakeReplayReport),
    Retired(IntakeReplayReport),
    RetirementMissing,
}

impl IntakeRoundReport {
    pub(super) fn record_route(&mut self, route: IntakeRouteReport) {
        match route {
            IntakeRouteReport::Active(report) => self.record_intake(report),
            IntakeRouteReport::RetirementPending(report) => {
                self.retirement_pending = self.retirement_pending.saturating_add(ROUTE_INCREMENT);
                self.record_intake(report);
            }
            IntakeRouteReport::Retired(report) => {
                self.retired = self.retired.saturating_add(ROUTE_INCREMENT);
                self.record_intake(report);
            }
            IntakeRouteReport::RetirementMissing => {
                self.routes = self.routes.saturating_add(ROUTE_INCREMENT);
                self.retirement_missing = self.retirement_missing.saturating_add(ROUTE_INCREMENT);
            }
        }
    }

    fn record_intake(&mut self, route: IntakeReplayReport) {
        self.routes = self.routes.saturating_add(ROUTE_INCREMENT);
        self.applied = self.applied.saturating_add(route.applied);
        self.replayed = self.replayed.saturating_add(route.replayed);
        self.stale = self.stale.saturating_add(route.stale);
        self.quarantined = self.quarantined.saturating_add(route.quarantined);
        self.deferred = self.deferred.saturating_add(route.deferred);
    }

    pub(super) fn record_failure(&mut self) {
        self.routes = self.routes.saturating_add(ROUTE_INCREMENT);
        self.failed_routes = self.failed_routes.saturating_add(ROUTE_INCREMENT);
    }

    pub(super) fn log(self) {
        if self.routes == usize::default() {
            return;
        }
        if self.has_warning() {
            tracing::warn!(
                routes = self.routes,
                failed_routes = self.failed_routes,
                applied = self.applied,
                replayed = self.replayed,
                stale = self.stale,
                quarantined = self.quarantined,
                deferred = self.deferred,
                retirement_pending = self.retirement_pending,
                retired = self.retired,
                retirement_missing = self.retirement_missing,
                "{}",
                super::super::constants::CODEX_INTAKE_ROUND_LOG
            );
        } else {
            tracing::trace!(
                routes = self.routes,
                applied = self.applied,
                replayed = self.replayed,
                stale = self.stale,
                retirement_pending = self.retirement_pending,
                retired = self.retired,
                retirement_missing = self.retirement_missing,
                "{}",
                super::super::constants::CODEX_INTAKE_ROUND_LOG
            );
        }
    }

    fn has_warning(self) -> bool {
        self.failed_routes > usize::default()
            || self.quarantined > usize::default()
            || self.deferred > usize::default()
    }
}
