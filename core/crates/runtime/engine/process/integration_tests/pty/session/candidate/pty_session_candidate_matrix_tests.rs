use super::*;
use anyhow::{Result, anyhow};
use std::cell::Cell;

const ENVIRONMENT_FAILURE_DETAIL: &str = "injected environment query failure";
const DESCRIPTOR_FAILURE_DETAIL: &str = "injected descriptor query failure";
const INVARIANT_FAILURE_DETAIL: &str = "injected invariant failure";
const GENERATION_FAILURE_DETAIL: &str = "injected generation query failure";
const UNEXPECTED_CALLBACK_DETAIL: &str = "excluded candidate callback must not run";
const INITIAL_GENERATION_CHECKS: usize = 0;
const GENERATION_CHECK_INCREMENT: usize = 1;
const ONE_GENERATION_CHECK: usize = 1;

#[test]
fn fenced_candidate_covers_generation_and_proof_outcomes() {
    let proof_called = Cell::new(false);
    let excluded = fenced_candidate(
        || Ok(false),
        || {
            proof_called.set(true);
            Ok(true)
        },
    )
    .unwrap();
    assert_eq!(excluded, CandidateEvaluation::Complete(false));
    assert!(!proof_called.get(), "{UNEXPECTED_CALLBACK_DETAIL}");

    assert_eq!(
        fenced_with(&[true, true], Ok(true)).unwrap(),
        CandidateEvaluation::Complete(true)
    );
    for (generations, proof) in [(&[true, false][..], true), (&[true, true][..], false)] {
        assert_eq!(
            fenced_with(generations, Ok(proof)).unwrap(),
            CandidateEvaluation::Complete(false)
        );
    }
    assert_eq!(
        fenced_with(&[true, false], local_bool_error()).unwrap(),
        CandidateEvaluation::Complete(false)
    );
    assert!(
        fenced_with(&[true, true], local_bool_error())
            .unwrap_err()
            .to_string()
            .contains(DESCRIPTOR_FAILURE_DETAIL)
    );
}

#[test]
fn fenced_candidate_preserves_generation_and_invariant_failures() {
    let checks = Cell::new(INITIAL_GENERATION_CHECKS);
    let error = fenced_candidate(
        || {
            checks.set(checks.get() + GENERATION_CHECK_INCREMENT);
            if checks.get() == ONE_GENERATION_CHECK {
                Ok(true)
            } else {
                Err(anyhow!(GENERATION_FAILURE_DETAIL))
            }
        },
        local_bool_error,
    )
    .unwrap_err();
    let detail = format!("{error:#}");
    assert!(detail.contains(GENERATION_FAILURE_DETAIL));
    assert!(detail.contains(DESCRIPTOR_FAILURE_DETAIL));
    assert!(detail.contains(CANDIDATE_GENERATION_FENCE_CONTEXT));

    let error = fenced_with(
        &[true],
        Err(CapabilityProofError::invariant(anyhow!(
            INVARIANT_FAILURE_DETAIL
        ))),
    )
    .unwrap_err();
    assert!(error.to_string().contains(INVARIANT_FAILURE_DETAIL));
}

#[test]
fn atomic_candidate_retries_an_active_descriptor_spawn_boundary() {
    assert_eq!(
        fenced_with(
            &[true],
            Err(CapabilityProofError::descriptor_scan_deferred()),
        )
        .unwrap(),
        CandidateEvaluation::Retry
    );
}

#[test]
fn capability_and_tty_short_circuit_matrix_is_explicit() {
    let tty_called = Cell::new(false);
    assert!(
        !match_tty_if_capability(EnvironmentCapability { matches: false }, || {
            tty_called.set(true);
            Ok(true)
        })
        .unwrap()
    );
    assert!(!tty_called.get(), "{UNEXPECTED_CALLBACK_DETAIL}");

    for tty in [false, true] {
        assert_eq!(
            match_tty_if_capability(EnvironmentCapability { matches: true }, || Ok(tty)).unwrap(),
            tty
        );
        assert_eq!(
            match_tty_after_environment(Ok(EnvironmentCapability { matches: true }), || Ok(tty))
                .unwrap(),
            tty
        );
    }
    assert!(
        match_tty_if_capability(EnvironmentCapability { matches: true }, local_bool_error).is_err()
    );
    assert!(
        match_tty_after_environment(local_environment_error(), || {
            Err(CapabilityProofError::invariant(anyhow!(
                INVARIANT_FAILURE_DETAIL
            )))
        })
        .is_err()
    );
}

#[test]
fn tty_first_environment_matrix_short_circuits_safely() {
    let environment_called = Cell::new(false);
    assert!(
        !match_environment_if_tty(false, || {
            environment_called.set(true);
            Ok(EnvironmentCapability { matches: true })
        })
        .unwrap()
    );
    assert!(!environment_called.get(), "{UNEXPECTED_CALLBACK_DETAIL}");
    assert!(
        match_environment_if_tty(true, || Ok(EnvironmentCapability { matches: true })).unwrap()
    );
    assert!(match_environment_if_tty(true, local_environment_error).is_err());
    assert!(
        match_tty_after_environment(
            Err(CapabilityProofError::invariant(anyhow!(
                INVARIANT_FAILURE_DETAIL
            ))),
            || Ok(true),
        )
        .is_err()
    );
}

fn fenced_with(
    generations: &[bool],
    proof: CapabilityProofResult<bool>,
) -> Result<CandidateEvaluation> {
    let index = Cell::new(INITIAL_GENERATION_CHECKS);
    fenced_candidate(
        || {
            let current = index.get();
            index.set(current + GENERATION_CHECK_INCREMENT);
            Ok(generations[current])
        },
        || proof,
    )
}

fn local_environment_error() -> CapabilityProofResult<EnvironmentCapability> {
    Err(CapabilityProofError::candidate(anyhow!(
        ENVIRONMENT_FAILURE_DETAIL
    )))
}

fn local_bool_error() -> CapabilityProofResult<bool> {
    Err(CapabilityProofError::candidate(anyhow!(
        DESCRIPTOR_FAILURE_DETAIL
    )))
}
