use super::*;

const ROOT_AUTHORITY_NOT_RETAINED_DETAIL: &str =
    "exact capability root match was not retained as internal authority";
const ROOT_PUBLISHED_DETAIL: &str = "capability root was published as descendant evidence";

#[test]
fn exact_root_match_is_retained_without_descendant_publication() {
    let root = child_candidate(THIRD_PID, THIRD_GENERATION, u64::default(), true);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let priorities = HashSet::from([root.live]);

    let result = advance::advance_with_priority_boundary(
        candidates([ambient, root]),
        &ReleasedScanState::default(),
        boundary::ReleasedPollBoundary::work_only(MAX_PTY_PROOF_CANDIDATES),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| Ok(CandidateEvaluation::Complete(candidate == root)),
    )
    .unwrap();

    assert!(
        result.candidates.observed.is_empty(),
        "{ROOT_PUBLISHED_DETAIL}"
    );
    assert!(
        result.candidates.stable.is_empty(),
        "{ROOT_PUBLISHED_DETAIL}"
    );
    assert!(
        result.candidates.identities.is_empty(),
        "{ROOT_PUBLISHED_DETAIL}"
    );
    assert!(
        result
            .advance
            .next
            .released
            .covers_exact_closure(&[root.live]),
        "{ROOT_AUTHORITY_NOT_RETAINED_DETAIL}"
    );
    assert_eq!(
        result.advance.next.released.pending_lineages_for_test(),
        [ambient.live.lineage_id()]
    );

    let next = advance::advance_with_priority_boundary(
        candidates([ambient, root]),
        &result.advance.next.released,
        boundary::ReleasedPollBoundary::work_only(MAX_PTY_PROOF_CANDIDATES),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    assert!(
        next.candidates.observed.is_empty(),
        "{ROOT_PUBLISHED_DETAIL}"
    );
    assert!(next.candidates.stable.is_empty(), "{ROOT_PUBLISHED_DETAIL}");
    assert!(
        next.candidates.identities.is_empty(),
        "{ROOT_PUBLISHED_DETAIL}"
    );
    assert!(
        next.advance
            .next
            .released
            .covers_exact_closure(&[root.live]),
        "{ROOT_AUTHORITY_NOT_RETAINED_DETAIL}"
    );
}
