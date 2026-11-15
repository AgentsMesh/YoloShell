use super::macos_candidate::CandidateEvaluation;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static INVENTORY_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static INVENTORY_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static INVENTORY_CAPTURE_MICROS: AtomicU64 = AtomicU64::new(0);
static EVALUATION_COMPLETE: AtomicU64 = AtomicU64::new(0);
static EVALUATION_DEFERRED: AtomicU64 = AtomicU64::new(0);
static EVALUATION_RETRY: AtomicU64 = AtomicU64::new(0);
static LIVE_EVALUATIONS: AtomicU64 = AtomicU64::new(0);
static FROZEN_EVALUATIONS: AtomicU64 = AtomicU64::new(0);
static FROZEN_POLLS: AtomicU64 = AtomicU64::new(0);
static FROZEN_ACTIVE_ROUND_POLLS: AtomicU64 = AtomicU64::new(0);
static FROZEN_BEGIN_REQUESTS: AtomicU64 = AtomicU64::new(0);
static FROZEN_EVENT_DEFERRALS: AtomicU64 = AtomicU64::new(0);
static FROZEN_ROUNDS_STARTED: AtomicU64 = AtomicU64::new(0);
static FROZEN_ROUNDS_CONCLUSIVE: AtomicU64 = AtomicU64::new(0);
static FROZEN_ROUNDS_INCONCLUSIVE: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_RESTARTS: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_CAPABILITY_GAPS: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_LINEAGE_GAPS: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_PUBLICATION_GAPS: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_COVERAGE_ONLY: AtomicU64 = AtomicU64::new(0);
static DISCOVERY_COVERED: AtomicU64 = AtomicU64::new(0);
static EVALUATION_MICROS: AtomicU64 = AtomicU64::new(0);
static EVALUATION_MAX_MICROS: AtomicU64 = AtomicU64::new(0);

pub(in crate::process) fn record_inventory_capture(cache_hit: bool, elapsed: Duration) {
    let counter = if cache_hit {
        &INVENTORY_CACHE_HITS
    } else {
        &INVENTORY_CACHE_MISSES
    };
    counter.fetch_add(1, Ordering::Relaxed);
    INVENTORY_CAPTURE_MICROS.fetch_add(micros(elapsed), Ordering::Relaxed);
}

pub(super) fn record_evaluation(frozen: bool, evaluation: CandidateEvaluation, elapsed: Duration) {
    let elapsed = micros(elapsed);
    if frozen {
        FROZEN_EVALUATIONS.fetch_add(1, Ordering::Relaxed);
    } else {
        LIVE_EVALUATIONS.fetch_add(1, Ordering::Relaxed);
    }
    let counter = match evaluation {
        CandidateEvaluation::Complete(_) => &EVALUATION_COMPLETE,
        CandidateEvaluation::Deferred => &EVALUATION_DEFERRED,
        CandidateEvaluation::Retry => &EVALUATION_RETRY,
    };
    counter.fetch_add(1, Ordering::Relaxed);
    EVALUATION_MICROS.fetch_add(elapsed, Ordering::Relaxed);
    EVALUATION_MAX_MICROS.fetch_max(elapsed, Ordering::Relaxed);
}

pub(super) fn record_poll(frozen: bool, active_round: bool) {
    if !frozen {
        return;
    }
    FROZEN_POLLS.fetch_add(1, Ordering::Relaxed);
    if active_round {
        FROZEN_ACTIVE_ROUND_POLLS.fetch_add(1, Ordering::Relaxed);
    }
}

pub(in crate::process) fn record_frozen_begin() {
    FROZEN_BEGIN_REQUESTS.fetch_add(1, Ordering::Relaxed);
}

pub(in crate::process) fn record_frozen_event_deferral() {
    FROZEN_EVENT_DEFERRALS.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn record_frozen_round_started() {
    FROZEN_ROUNDS_STARTED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn record_frozen_round_completed(conclusive: bool) {
    let counter = if conclusive {
        &FROZEN_ROUNDS_CONCLUSIVE
    } else {
        &FROZEN_ROUNDS_INCONCLUSIVE
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub(in crate::process) fn record_discovery_gap(capability: bool, lineage: bool, publication: bool) {
    DISCOVERY_CAPABILITY_GAPS.fetch_add(u64::from(!capability), Ordering::Relaxed);
    DISCOVERY_LINEAGE_GAPS.fetch_add(u64::from(!lineage), Ordering::Relaxed);
    DISCOVERY_PUBLICATION_GAPS.fetch_add(u64::from(!publication), Ordering::Relaxed);
}

pub(in crate::process) fn record_discovery_coverage(covered: bool) {
    let counter = if covered {
        &DISCOVERY_COVERED
    } else {
        &DISCOVERY_COVERAGE_ONLY
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub(in crate::process) fn record_discovery_restart() {
    DISCOVERY_RESTARTS.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn snapshot() -> String {
    format!(
        "cache_hits={} cache_misses={} inventory_us={} eval_complete={} eval_deferred={} \
         eval_retry={} live_eval={} frozen_eval={} frozen_polls={} frozen_active_round_polls={} \
         frozen_begin={} frozen_event_deferrals={} frozen_rounds_started={} frozen_conclusive={} \
         frozen_inconclusive={} discovery_restarts={} discovery_capability_gaps={} \
         discovery_lineage_gaps={} discovery_publication_gaps={} discovery_coverage_only={} \
         discovery_covered={} \
         eval_us={} eval_max_us={}",
        INVENTORY_CACHE_HITS.load(Ordering::Relaxed),
        INVENTORY_CACHE_MISSES.load(Ordering::Relaxed),
        INVENTORY_CAPTURE_MICROS.load(Ordering::Relaxed),
        EVALUATION_COMPLETE.load(Ordering::Relaxed),
        EVALUATION_DEFERRED.load(Ordering::Relaxed),
        EVALUATION_RETRY.load(Ordering::Relaxed),
        LIVE_EVALUATIONS.load(Ordering::Relaxed),
        FROZEN_EVALUATIONS.load(Ordering::Relaxed),
        FROZEN_POLLS.load(Ordering::Relaxed),
        FROZEN_ACTIVE_ROUND_POLLS.load(Ordering::Relaxed),
        FROZEN_BEGIN_REQUESTS.load(Ordering::Relaxed),
        FROZEN_EVENT_DEFERRALS.load(Ordering::Relaxed),
        FROZEN_ROUNDS_STARTED.load(Ordering::Relaxed),
        FROZEN_ROUNDS_CONCLUSIVE.load(Ordering::Relaxed),
        FROZEN_ROUNDS_INCONCLUSIVE.load(Ordering::Relaxed),
        DISCOVERY_RESTARTS.load(Ordering::Relaxed),
        DISCOVERY_CAPABILITY_GAPS.load(Ordering::Relaxed),
        DISCOVERY_LINEAGE_GAPS.load(Ordering::Relaxed),
        DISCOVERY_PUBLICATION_GAPS.load(Ordering::Relaxed),
        DISCOVERY_COVERAGE_ONLY.load(Ordering::Relaxed),
        DISCOVERY_COVERED.load(Ordering::Relaxed),
        EVALUATION_MICROS.load(Ordering::Relaxed),
        EVALUATION_MAX_MICROS.load(Ordering::Relaxed),
    )
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}
