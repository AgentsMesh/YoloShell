use super::super::authority::LifecycleAuthority;
use super::super::authority_control::CodexAuthorityControl;
use super::super::authority_registry::{CodexAuthorityTakeover, CodexCurrentAuthority};
use super::build::RuntimeBuildContext;
use super::{CodexHookRuntime, ReplayReport};
use crate::codex_hooks::CodexSnapshotLineage;
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{Arc, MutexGuard};

struct RuntimeCandidate {
    artifacts: crate::codex_hooks::CodexHookArtifacts,
    outbox: crate::codex_hooks::CodexOutbox,
    lifecycle: LifecycleAuthority,
    report: ReplayReport,
}

pub(super) fn build(
    context: RuntimeBuildContext<'_>,
) -> Result<(Arc<CodexHookRuntime>, ReplayReport)> {
    context.verify_generation()?;
    let key = super::super::authority_registry::CodexAuthorityKey::new(
        context.terminal_id,
        context.runtime_epoch,
        context.owner.invocation_id,
    );
    let registry = Arc::clone(&context.registry);
    registry.with_takeover(key, |takeover| {
        let predecessor = takeover.current().cloned();
        match predecessor {
            Some(predecessor) => build_successor(&context, takeover, predecessor),
            None => {
                let candidate = restore_candidate(&context, None)?;
                finish(&context, takeover, candidate, None)
            }
        }
    })
}

fn build_successor(
    context: &RuntimeBuildContext<'_>,
    takeover: CodexAuthorityTakeover<'_>,
    predecessor: CodexCurrentAuthority,
) -> Result<(Arc<CodexHookRuntime>, ReplayReport)> {
    let control = Arc::clone(predecessor.control());
    let mut lifecycle = control.lock_for_takeover()?;
    let lineage = lifecycle.snapshot_lineage()?;
    let result = catch_unwind(AssertUnwindSafe(|| {
        let candidate = restore_candidate(context, Some(lineage))?;
        finish(
            context,
            takeover,
            candidate,
            Some((&predecessor, &mut lifecycle)),
        )
    }));
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => {
            control.contain_failed_takeover(&mut lifecycle);
            Err(error)
        }
        Err(payload) => {
            control.contain_failed_takeover(&mut lifecycle);
            resume_unwind(payload)
        }
    }
}

fn restore_candidate(
    context: &RuntimeBuildContext<'_>,
    lineage: Option<CodexSnapshotLineage>,
) -> Result<RuntimeCandidate> {
    context.verify_generation()?;
    context.mode.verify(&context.owner)?;
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        context.paths,
        context.terminal_id,
        context.runtime_epoch,
        context.owner.invocation_id,
    )?;
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(artifacts.outbox_path(), &context.owner)?;
    let snapshot_path = artifacts.snapshot_path();
    let (lifecycle, report) =
        super::super::outbox_authority::with_locked_state(&outbox, |outbox| {
            let (cursor, snapshot) = crate::codex_hooks::CodexSnapshotCursor::open(&snapshot_path)?;
            if let Some(lineage) = lineage {
                cursor.require_lineage(lineage)?;
            }
            admit_if_required(context, snapshot.as_ref())?;
            super::super::replay::restore(
                context.owner.clone(),
                context.provider_version,
                cursor,
                snapshot,
                outbox,
            )
        })?;
    context.mode.verify(&context.owner)?;
    Ok(RuntimeCandidate {
        artifacts,
        outbox,
        lifecycle,
        report,
    })
}

fn admit_if_required(
    context: &RuntimeBuildContext<'_>,
    snapshot: Option<&crate::agent::hooks::ReducerSnapshot>,
) -> Result<()> {
    context.mode.verify(&context.owner)?;
    context.verify_generation()?;
    if !context.mode.requires_live_admission() {
        return Ok(());
    }
    if !super::super::replay::requires_live_admission(
        &context.owner,
        context.provider_version,
        snapshot,
    )? {
        return Ok(());
    }
    let result = super::super::admission::admit_process_source(
        context.process_source.as_ref(),
        context.shell,
        crate::process::ProcessIdentity {
            pid: context.owner.process.pid,
            start_time: context.owner.process.start_time,
        },
        context.owner.process.pid,
    )
    .map_err(anyhow::Error::new);
    context.verify_generation()?;
    result
}

fn finish(
    context: &RuntimeBuildContext<'_>,
    takeover: CodexAuthorityTakeover<'_>,
    candidate: RuntimeCandidate,
    predecessor: Option<(
        &CodexCurrentAuthority,
        &mut MutexGuard<'_, LifecycleAuthority>,
    )>,
) -> Result<(Arc<CodexHookRuntime>, ReplayReport)> {
    let RuntimeCandidate {
        artifacts,
        outbox,
        lifecycle,
        report,
    } = candidate;
    context.mode.verify(&context.owner)?;
    context.verify_generation()?;
    let _generation_fence = context.acquire_generation_fence()?;
    let initial = context.mode.initial_view(&lifecycle)?;
    let control = CodexAuthorityControl::new(lifecycle);
    let claim = {
        let mut lifecycle = control.lock_for_takeover()?;
        lifecycle.claim_snapshot_with(|proof| {
            super::build_support::run_before_initial_publish_hook();
            control.publish_initial(proof, initial)
        })?
    };
    context.verify_generation()?;
    let prepared = takeover.prepare_registration(claim, Arc::clone(&control));
    let runtime = Arc::new(CodexHookRuntime {
        registration: prepared.bind(),
        terminal_id: context.terminal_id,
        runtime_epoch: context.runtime_epoch,
        shell: context.shell,
        owner: context.owner.clone(),
        artifacts,
        outbox,
        process_source: Arc::clone(&context.process_source),
        #[cfg(test)]
        faults: Default::default(),
    });
    let succession = takeover.install(&runtime.registration);
    if let (Some((predecessor, lifecycle)), Some(proof)) = (predecessor, succession) {
        predecessor.control().supersede_locked(lifecycle, proof);
    }
    Ok((runtime, report))
}
