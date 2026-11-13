use super::route::ProjectionRoute;
use crate::agent::codex::authority::CodexAuthorityVersion;
use crate::agent::hooks::{EventSequence, NormalizedLifecycle};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CursorDecision {
    Apply,
    Conflict,
    Replay,
    Stale,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ProjectionPhase {
    Active,
    ConfirmedExit,
    Exhausted,
    Terminal,
}

#[derive(Clone, Copy)]
struct ProjectionMark {
    version: CodexAuthorityVersion,
    phase: ProjectionPhase,
}

#[derive(Default)]
pub(super) struct ProjectionCursor {
    current: Option<ProjectionMark>,
}

impl ProjectionCursor {
    pub(super) fn lifecycle_version(
        route: &ProjectionRoute,
        lifecycle: &NormalizedLifecycle,
        authority_revision: u64,
    ) -> CodexAuthorityVersion {
        CodexAuthorityVersion {
            route_revision: route.revision,
            invocation_id: route.key.invocation_id(),
            generation: lifecycle.generation,
            authority_revision,
            sequence: lifecycle.last_sequence.0,
        }
    }

    pub(super) fn terminal_version(&self, route: &ProjectionRoute) -> CodexAuthorityVersion {
        self.current
            .map(|mark| mark.version)
            .filter(|current| same_route(*current, route))
            .unwrap_or(CodexAuthorityVersion {
                route_revision: route.revision,
                invocation_id: route.key.invocation_id(),
                generation: route.admission.record().request().identity.generation,
                authority_revision: u64::default(),
                sequence: EventSequence::ZERO.0,
            })
    }

    pub(super) fn classify(
        &self,
        next: CodexAuthorityVersion,
        phase: ProjectionPhase,
    ) -> CursorDecision {
        let Some(current) = self.current else {
            return CursorDecision::Apply;
        };
        match compare(current.version, next) {
            None => CursorDecision::Conflict,
            Some(Ordering::Greater) => CursorDecision::Stale,
            Some(Ordering::Equal) if current.phase == phase => CursorDecision::Replay,
            Some(Ordering::Equal) if current.phase < phase => CursorDecision::Apply,
            Some(Ordering::Equal) => CursorDecision::Stale,
            Some(Ordering::Less) => CursorDecision::Apply,
        }
    }

    pub(super) fn preserves_closed(&self, route: &ProjectionRoute) -> bool {
        self.current.is_some_and(|mark| {
            same_route(mark.version, route) && mark.phase == ProjectionPhase::ConfirmedExit
        })
    }

    pub(super) fn advance(&mut self, next: CodexAuthorityVersion, phase: ProjectionPhase) {
        self.current = Some(ProjectionMark {
            version: next,
            phase,
        });
    }
}

pub(super) fn compare(
    left: CodexAuthorityVersion,
    right: CodexAuthorityVersion,
) -> Option<Ordering> {
    match left.route_revision.cmp(&right.route_revision) {
        Ordering::Equal if left.invocation_id != right.invocation_id => None,
        Ordering::Equal => compare_generation(left, right),
        ordering => Some(ordering),
    }
}

fn compare_generation(
    left: CodexAuthorityVersion,
    right: CodexAuthorityVersion,
) -> Option<Ordering> {
    match left.generation.cmp(&right.generation) {
        Ordering::Equal => compare_revision(left, right),
        ordering => Some(ordering),
    }
}

fn compare_revision(left: CodexAuthorityVersion, right: CodexAuthorityVersion) -> Option<Ordering> {
    match left.authority_revision.cmp(&right.authority_revision) {
        Ordering::Equal if left.sequence != right.sequence => None,
        Ordering::Equal => Some(Ordering::Equal),
        Ordering::Less if left.sequence <= right.sequence => Some(Ordering::Less),
        Ordering::Greater if left.sequence >= right.sequence => Some(Ordering::Greater),
        Ordering::Less | Ordering::Greater => None,
    }
}

fn same_route(version: CodexAuthorityVersion, route: &ProjectionRoute) -> bool {
    version.route_revision == route.revision && version.invocation_id == route.key.invocation_id()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_cursor_revision_tests.rs"]
mod revision_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_cursor_tests.rs"]
mod tests;
