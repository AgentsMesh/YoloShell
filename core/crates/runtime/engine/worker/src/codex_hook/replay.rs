use super::authority::{CommittedReduction, LifecycleAuthority};
use super::constants::{
    CODEX_OUTBOX_HISTORY_ERROR, CODEX_OUTBOX_TIP_ERROR, CODEX_REDUCER_ERROR, CODEX_REPLAY_ERROR,
    CODEX_SNAPSHOT_WATERMARK_ERROR,
};
use super::marker_tail;
use super::publication::ReplayReport;
use crate::agent::hooks::{
    ApplyDisposition, HookIdentity, ProcessState, HookProviderKind, ProviderVersion, Reducer,
    ReducerSnapshot,
};
use crate::codex_hooks::{CodexOutboxState, CodexSnapshotCursor};
use anyhow::{Context, Result, ensure};

pub(super) fn restore(
    owner: HookIdentity,
    provider_version: ProviderVersion,
    cursor: CodexSnapshotCursor,
    snapshot: Option<ReducerSnapshot>,
    outbox: &CodexOutboxState,
) -> Result<(LifecycleAuthority, ReplayReport)> {
    let snapshot_present = snapshot.is_some();
    let reducer = match snapshot {
        Some(snapshot) => Reducer::restore(
            snapshot,
            owner.clone(),
            HookProviderKind::Codex,
            provider_version,
        )
        .context(CODEX_REPLAY_ERROR)?,
        None => Reducer::new(owner.clone(), HookProviderKind::Codex, provider_version)
            .map_err(anyhow::Error::new)
            .context(CODEX_REDUCER_ERROR)?,
    };
    let mut authority = LifecycleAuthority::new(owner, reducer, cursor);
    validate_snapshot_watermark(&authority, outbox, snapshot_present)?;
    let report = if authority.authority_exhausted() {
        validate_authority(&authority, outbox)?;
        ReplayReport::default()
    } else {
        match reconcile(&mut authority, outbox) {
            Ok(report) => report,
            Err(error)
                if super::authority::is_authority_exhausted_error(&error)
                    && authority.authority_exhausted() =>
            {
                ReplayReport::default()
            }
            Err(error) => return Err(error),
        }
    };
    Ok((authority, report))
}

pub(super) fn requires_live_admission(
    owner: &HookIdentity,
    provider_version: ProviderVersion,
    snapshot: Option<&ReducerSnapshot>,
) -> Result<bool> {
    let Some(snapshot) = snapshot else {
        return Ok(true);
    };
    let reducer = Reducer::restore(
        snapshot.clone(),
        owner.clone(),
        HookProviderKind::Codex,
        provider_version,
    )
    .context(CODEX_REPLAY_ERROR)?;
    Ok(!reducer.authority_exhausted() && reducer.state().process == ProcessState::Alive)
}

pub(super) fn reconcile(
    authority: &mut LifecycleAuthority,
    outbox: &CodexOutboxState,
) -> Result<ReplayReport> {
    authority.ensure_operable()?;
    let range = validate_authority(authority, outbox)?;
    let tip = range.tip;
    let installed = range.installed;
    if installed == tip {
        return Ok(ReplayReport::default());
    }
    let expected = installed
        .checked_add(crate::agent::hooks::FIRST_SEQUENCE)
        .context(CODEX_OUTBOX_HISTORY_ERROR)?;
    let suffix: Vec<_> = marker_tail::retained_events(outbox)
        .filter(|event| event.sequence.0 > installed)
        .cloned()
        .collect();
    ensure!(
        suffix.first().map(|event| event.sequence.0) == Some(expected)
            && suffix.last().map(|event| event.sequence.0) == Some(tip),
        CODEX_OUTBOX_HISTORY_ERROR
    );
    if authority.reducer()?.is_revoked() {
        return Ok(ReplayReport::default());
    }
    let mut report = ReplayReport::default();
    for event in suffix {
        let marker = outbox.durable_rejection_for(&event)?;
        let sequence = event.sequence.0;
        let reduction = super::reduction_probe::commit(authority, event, marker)?;
        match reduction {
            CommittedReduction::Accepted(publication) => {
                ensure!(marker.is_none(), CODEX_OUTBOX_HISTORY_ERROR);
                report.record(publication.disposition);
            }
            CommittedReduction::Rejected(violation, state) => {
                let marker = marker.context(CODEX_OUTBOX_HISTORY_ERROR)?;
                ensure!(marker.violation() == violation, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(state.revoked, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(sequence == tip, CODEX_OUTBOX_HISTORY_ERROR);
                return Ok(report);
            }
        }
    }
    Ok(report)
}

struct ReplayRange {
    tip: u64,
    installed: u64,
}

pub(super) fn validate_restored(
    authority: &LifecycleAuthority,
    outbox: &CodexOutboxState,
) -> Result<()> {
    validate_authority(authority, outbox).map(|_| ())
}

fn validate_authority(
    authority: &LifecycleAuthority,
    outbox: &CodexOutboxState,
) -> Result<ReplayRange> {
    let tip = outbox
        .next_sequence()
        .checked_sub(crate::agent::hooks::FIRST_SEQUENCE)
        .context(CODEX_OUTBOX_TIP_ERROR)?;
    let installed = authority.reducer()?.state().last_sequence.0;
    ensure!(installed <= tip, CODEX_OUTBOX_TIP_ERROR);
    crate::codex_hooks::validate_outbox_watermark(outbox, installed)
        .context(CODEX_SNAPSHOT_WATERMARK_ERROR)?;
    marker_tail::validate_owner(
        outbox,
        authority.owner(),
        tip != crate::agent::hooks::ZERO_SEQUENCE,
    )?;
    marker_tail::validate(outbox, authority.reducer()?, installed, tip)?;
    marker_tail::validate_overlap(outbox, authority.reducer()?, installed)?;
    Ok(ReplayRange { tip, installed })
}

fn validate_snapshot_watermark(
    authority: &LifecycleAuthority,
    outbox: &CodexOutboxState,
    snapshot_present: bool,
) -> Result<()> {
    if !snapshot_present {
        ensure!(
            outbox.acknowledged_sequence() == crate::agent::hooks::ZERO_SEQUENCE,
            CODEX_SNAPSHOT_WATERMARK_ERROR
        );
    }
    let installed = authority.reducer()?.state().last_sequence.0;
    crate::codex_hooks::validate_outbox_watermark(outbox, installed)
        .context(CODEX_SNAPSHOT_WATERMARK_ERROR)?;
    Ok(())
}

impl ReplayReport {
    fn record(&mut self, disposition: ApplyDisposition) {
        match disposition {
            ApplyDisposition::Applied => self.applied += 1,
            ApplyDisposition::Replay => self.replayed += 1,
            ApplyDisposition::Stale => self.stale += 1,
            ApplyDisposition::Invalidated => {}
        }
    }

    pub(super) fn merge(&mut self, other: Self) {
        self.applied += other.applied;
        self.replayed += other.replayed;
        self.stale += other.stale;
    }
}
