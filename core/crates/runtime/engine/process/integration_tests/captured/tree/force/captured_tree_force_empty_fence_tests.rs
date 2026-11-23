use super::FrozenKillBasis;
use super::ProcessClosureVersion;
use super::commit::{
    closure_observations_match, typed_closure_observations_match, validate_output_fence_version,
};
use super::empty::{assert_gone_proof_version, empty_closure_can_report_gone};
use crate::process::captured_tree::freeze::FreezeClosureCandidate;
use crate::process::inventory_test_support::TestProcess;
use crate::process::{DescendantObservation, LiveProcessIdentity};

const NEW_DESCENDANT_ACCEPTED_DETAIL: &str =
    "new descendant matched an empty frozen process closure";
const PENDING_SCAN_ACCEPTED_DETAIL: &str =
    "pending capability scan matched an empty frozen process closure";
const INDETERMINATE_LINEAGE_ACCEPTED_DETAIL: &str =
    "indeterminate lineage matched an empty frozen process closure";
const STABLE_EMPTY_REJECTED_DETAIL: &str = "two empty inspections did not match the empty closure";
const STALE_OUTPUT_VERSION_ACCEPTED_DETAIL: &str = "stale output-fence version was accepted";
const STALE_GONE_TOKEN_ACCEPTED_DETAIL: &str = "stale producer-gone token was accepted";
const FOREIGN_TREE_TOKEN_ACCEPTED_DETAIL: &str =
    "same-generation token from another process tree was accepted";
const BASIS_DRIFT_ACCEPTED_DETAIL: &str =
    "ordinary closure observation validated exact-positive authority";
const BASIS_STRENGTHENING_REJECTED_DETAIL: &str =
    "stronger final observation did not preserve conservative frozen authority";
const BASIS_DOWNGRADE_ACCEPTED_DETAIL: &str =
    "weaker final observation validated stronger frozen authority";

#[test]
fn empty_after_freeze_rejects_a_new_descendant() {
    let descendant = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(descendant.identity)
        .unwrap()
        .unwrap();
    let observed = [live];

    assert!(
        !closure_observations_match(&[], Some(&observed), Some(&observed)),
        "{NEW_DESCENDANT_ACCEPTED_DETAIL}"
    );
}

#[test]
fn capability_scan_pending_rejects_each_empty_confirmation_position() {
    let empty: Option<&[LiveProcessIdentity]> = Some(&[]);

    assert!(
        !closure_observations_match(&[], None, empty),
        "{PENDING_SCAN_ACCEPTED_DETAIL}"
    );
    assert!(
        !closure_observations_match(&[], empty, None),
        "{PENDING_SCAN_ACCEPTED_DETAIL}"
    );
}

#[test]
fn lineage_indeterminate_rejects_the_empty_closure_fence() {
    let empty: Option<&[LiveProcessIdentity]> = Some(&[]);

    assert!(
        !closure_observations_match(&[], empty, None),
        "{INDETERMINATE_LINEAGE_ACCEPTED_DETAIL}"
    );
}

#[test]
fn two_fresh_empty_inspections_accept_the_current_empty_closure() {
    let empty: Option<&[LiveProcessIdentity]> = Some(&[]);

    assert!(
        closure_observations_match(&[], empty, empty),
        "{STABLE_EMPTY_REJECTED_DETAIL}"
    );
}

#[test]
fn exact_confirmation_rejects_same_closure_with_ordinary_basis() {
    let process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let exact = FreezeClosureCandidate::Exact(vec![live.clone()]);
    let ordinary = FreezeClosureCandidate::Ordinary(vec![live.clone()]);

    assert_ne!(exact, ordinary, "{BASIS_DRIFT_ACCEPTED_DETAIL}");
    assert!(
        !typed_closure_observations_match(
            &[live],
            FrozenKillBasis::ExactPositive,
            Some(&exact),
            Some(&ordinary),
        ),
        "{BASIS_DRIFT_ACCEPTED_DETAIL}"
    );
}

#[test]
fn final_revalidation_uses_an_explicit_one_way_basis_matrix() {
    let process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let expected = [live.clone()];
    let exact = FreezeClosureCandidate::Exact(vec![live.clone()]);
    let incremental = FreezeClosureCandidate::IncrementalExact(vec![live.clone()]);
    let ordinary = FreezeClosureCandidate::Ordinary(vec![live.clone()]);
    let kernel_eof = FreezeClosureCandidate::KernelEof(vec![live]);
    let matches = |basis, candidate| {
        typed_closure_observations_match(&expected, basis, Some(candidate), Some(candidate))
    };

    assert!(
        matches(FrozenKillBasis::IncrementalExactPositive, &exact)
            && matches(FrozenKillBasis::Ordinary, &exact)
            && matches(FrozenKillBasis::KernelEof, &kernel_eof),
        "{BASIS_STRENGTHENING_REJECTED_DETAIL}"
    );
    assert!(
        !matches(FrozenKillBasis::ExactPositive, &incremental)
            && !matches(FrozenKillBasis::IncrementalExactPositive, &ordinary)
            && !matches(FrozenKillBasis::Ordinary, &kernel_eof)
            && !matches(FrozenKillBasis::KernelEof, &exact)
            && !matches(FrozenKillBasis::KernelEof, &incremental)
            && !matches(FrozenKillBasis::KernelEof, &ordinary),
        "{BASIS_DOWNGRADE_ACCEPTED_DETAIL}"
    );
}

#[test]
fn writer_present_coverage_cannot_report_an_empty_closure_gone() {
    assert!(!empty_closure_can_report_gone(
        DescendantObservation::CapabilityScanPending
    ));
    assert!(!empty_closure_can_report_gone(
        DescendantObservation::Indeterminate
    ));
    assert!(empty_closure_can_report_gone(
        DescendantObservation::Settled
    ));
}

#[test]
fn prior_closure_versions_cannot_authorize_kill_or_gone_commit() {
    let previous = ProcessClosureVersion::default();
    let current = previous.next().unwrap();

    assert!(
        validate_output_fence_version(&current, &previous).is_err(),
        "{STALE_OUTPUT_VERSION_ACCEPTED_DETAIL}"
    );
    let rejection = std::panic::catch_unwind(|| assert_gone_proof_version(current, previous));
    assert!(rejection.is_err(), "{STALE_GONE_TOKEN_ACCEPTED_DETAIL}");
}

#[test]
fn same_generation_from_another_tree_cannot_authorize_commit() {
    let current = ProcessClosureVersion::default().next().unwrap();
    let foreign = ProcessClosureVersion::default().next().unwrap();

    assert!(
        validate_output_fence_version(&current, &foreign).is_err(),
        "{FOREIGN_TREE_TOKEN_ACCEPTED_DETAIL}"
    );
    let rejection = std::panic::catch_unwind(|| assert_gone_proof_version(current, foreign));
    assert!(rejection.is_err(), "{FOREIGN_TREE_TOKEN_ACCEPTED_DETAIL}");
}
