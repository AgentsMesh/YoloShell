use super::FrozenKillBasis;
use super::commit::typed_closure_observations_match;
use crate::process::LiveProcessIdentity;
use crate::process::captured_tree::freeze::FreezeClosureCandidate;
use crate::process::inventory_test_support::TestProcess;

const MATRIX_MISMATCH_DETAIL: &str = "typed final-revalidation matrix mismatch";
const NONE_ACCEPTED_DETAIL: &str = "missing final observation was accepted";
const CLOSURE_MISMATCH_ACCEPTED_DETAIL: &str = "foreign closure matched final revalidation";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateKind {
    Exact,
    Incremental,
    Ordinary,
    KernelEof,
}

const CANDIDATE_KINDS: &[CandidateKind] = &[
    CandidateKind::Exact,
    CandidateKind::Incremental,
    CandidateKind::Ordinary,
    CandidateKind::KernelEof,
];
const BASIS_CASES: &[(FrozenKillBasis, &str)] = &[
    (FrozenKillBasis::ExactPositive, "exact"),
    (FrozenKillBasis::IncrementalExactPositive, "incremental"),
    (FrozenKillBasis::Ordinary, "ordinary"),
    (FrozenKillBasis::KernelEof, "kernel_eof"),
];

#[test]
fn final_revalidation_requires_two_identical_compatible_candidates() {
    let process = TestProcess::spawn();
    let live = live_identity(&process);
    for &(basis, basis_name) in BASIS_CASES {
        for &first_kind in CANDIDATE_KINDS {
            for &second_kind in CANDIDATE_KINDS {
                let first = candidate(first_kind, &live);
                let second = candidate(second_kind, &live);
                let expected = first_kind == second_kind && accepts(basis, first_kind);
                let actual = typed_closure_observations_match(
                    std::slice::from_ref(&live),
                    basis,
                    Some(&first),
                    Some(&second),
                );
                assert_eq!(
                    actual, expected,
                    "{MATRIX_MISMATCH_DETAIL}: basis={basis_name}, first={first_kind:?}, second={second_kind:?}"
                );
            }
        }
    }
}

#[test]
fn final_revalidation_rejects_missing_and_foreign_closures() {
    let expected_process = TestProcess::spawn();
    let foreign_process = TestProcess::spawn();
    let expected = live_identity(&expected_process);
    let foreign = live_identity(&foreign_process);
    let exact = candidate(CandidateKind::Exact, &expected);
    let foreign_exact = candidate(CandidateKind::Exact, &foreign);

    assert!(
        !typed_closure_observations_match(
            std::slice::from_ref(&expected),
            FrozenKillBasis::ExactPositive,
            None,
            Some(&exact),
        ),
        "{NONE_ACCEPTED_DETAIL}"
    );
    assert!(
        !typed_closure_observations_match(
            std::slice::from_ref(&expected),
            FrozenKillBasis::ExactPositive,
            Some(&exact),
            None,
        ),
        "{NONE_ACCEPTED_DETAIL}"
    );
    assert!(
        !typed_closure_observations_match(
            std::slice::from_ref(&expected),
            FrozenKillBasis::ExactPositive,
            Some(&foreign_exact),
            Some(&foreign_exact),
        ),
        "{CLOSURE_MISMATCH_ACCEPTED_DETAIL}"
    );
}

fn accepts(basis: FrozenKillBasis, candidate: CandidateKind) -> bool {
    match basis {
        FrozenKillBasis::ExactPositive => candidate == CandidateKind::Exact,
        FrozenKillBasis::IncrementalExactPositive => {
            matches!(candidate, CandidateKind::Exact | CandidateKind::Incremental)
        }
        FrozenKillBasis::Ordinary => candidate != CandidateKind::KernelEof,
        FrozenKillBasis::KernelEof => candidate == CandidateKind::KernelEof,
    }
}

fn candidate(kind: CandidateKind, identity: &LiveProcessIdentity) -> FreezeClosureCandidate {
    let closure = vec![identity.clone()];
    match kind {
        CandidateKind::Exact => FreezeClosureCandidate::Exact(closure),
        CandidateKind::Incremental => FreezeClosureCandidate::IncrementalExact(closure),
        CandidateKind::Ordinary => FreezeClosureCandidate::Ordinary(closure),
        CandidateKind::KernelEof => FreezeClosureCandidate::KernelEof(closure),
    }
}

fn live_identity(process: &TestProcess) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap()
}
