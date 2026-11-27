use crate::process::ReservedDetachedChildSettlement;
use anyhow::Context;
use std::process::{Child, Command};

const ACTIVE_CHILD_EXPECTATION: &str = "guardian spawn settlement must retain an active child";

#[derive(Clone, Copy, Eq, PartialEq)]
enum PublicationState {
    Impossible,
    Possible,
}

struct RetainedGuardian {
    child: Child,
    settlement: ReservedDetachedChildSettlement,
}

pub(super) struct GuardianSpawnSettlement {
    retained: Option<RetainedGuardian>,
    publication: PublicationState,
}

impl GuardianSpawnSettlement {
    pub(super) fn spawn(command: &mut Command) -> anyhow::Result<Self> {
        reserve_then_spawn(crate::process::reserve_detached_child_settlement, || {
            let spawn_guard = crate::process::waitable_child_spawn_guard()?;
            spawn_guard
                .boundary()
                .spawn_command(command)
                .map_err(Into::into)
        })
    }

    pub(super) fn child(&self) -> anyhow::Result<&Child> {
        self.retained
            .as_ref()
            .map(|retained| &retained.child)
            .context(ACTIVE_CHILD_EXPECTATION)
    }

    pub(super) fn mark_publication_possible(&mut self) {
        self.publication = PublicationState::Possible;
    }

    pub(super) fn retain_naturally(mut self) -> anyhow::Result<()> {
        let retained = self.retained.take().context(ACTIVE_CHILD_EXPECTATION)?;
        retained.settlement.retain(retained.child);
        Ok(())
    }
}

impl Drop for GuardianSpawnSettlement {
    fn drop(&mut self) {
        let Some(retained) = self.retained.take() else {
            return;
        };
        match self.publication {
            PublicationState::Impossible => retained.settlement.terminate(retained.child),
            PublicationState::Possible => retained.settlement.retain(retained.child),
        }
    }
}

fn reserve_then_spawn(
    reserve: impl FnOnce() -> anyhow::Result<ReservedDetachedChildSettlement>,
    spawn: impl FnOnce() -> anyhow::Result<Child>,
) -> anyhow::Result<GuardianSpawnSettlement> {
    let settlement = reserve()?;
    let child = spawn()?;
    Ok(GuardianSpawnSettlement {
        retained: Some(RetainedGuardian { child, settlement }),
        publication: PublicationState::Impossible,
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/spawn/spawn_settlement_tests.rs"]
mod tests;
