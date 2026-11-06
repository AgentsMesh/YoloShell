use super::*;
use crate::process::captured_tree_macos_wait::TEST_TIMEOUT;
use crate::process::constants::NO_PARENT_LINEAGE_ID;
use crate::process::inventory_observation::cache;
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, entry};
use crate::process::pty_session_proof::PtyCapabilityScanState;
use crate::process::{MacosProcessCapabilityIssuer, ProcessIdentity};
use std::time::Instant;

const TEST_CAPABILITY: &str = "released-spawn-retry";
const SINGLE_CANDIDATE_DETAIL: &str = "spawn retry fixture must contain one candidate";
const COMPLETION_TIMEOUT_DETAIL: &str =
    "descriptor spawn retry did not complete before the deadline";

#[test]
fn descriptor_spawn_retry_retains_frozen_inventory_until_exact_completion() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let durable = ProcessIdentity::capture(std::process::id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    authority.verify_writer_holder(&live).unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(authority).unwrap();
    let process = entry(durable, NO_PARENT_PID, durable.pid);
    let _inventory = install_repeating_test_inventory(vec![process]);
    let mode = MacosGlobalScanMode::TransferableWriterPresent;
    let context = GlobalCandidateContext::Escaped;
    let mut state = PtyCapabilityScanState::default();
    let frozen = cache::capture(&state).unwrap();
    let current =
        scan_candidate_universe(&frozen, mode, u64::default(), NO_PARENT_LINEAGE_ID).unwrap();
    let [candidate] = current.as_ref() else {
        panic!("{SINGLE_CANDIDATE_DETAIL}");
    };
    assert_eq!(candidate.live, live);

    let spawn_gate = crate::process::waitable_child_spawn_guard().unwrap();
    let mut first_evaluation = None;
    let mut first = advance_with_policy(
        Arc::clone(&current),
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |candidate, budget| {
            let evaluation = evaluate_global_candidate(&proof, candidate, budget, context, mode)?;
            first_evaluation = Some((candidate.live, evaluation));
            Ok(evaluation)
        },
    )
    .unwrap();

    assert_eq!(first_evaluation, Some((live, CandidateEvaluation::Retry)));
    assert_eq!(first.scheduling_status, CapabilityScanStatus::Pending);
    assert_eq!(first.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(
        first.advance.next.released_pending_lineages_for_test(),
        [live.lineage_id()]
    );
    cache::retain(&mut first, &frozen);
    let retained = first.advance.next.frozen_inventory().unwrap();
    assert!(Arc::ptr_eq(&frozen, &retained));
    state.commit(first.advance);

    drop(spawn_gate);
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let reused = cache::capture(&state).unwrap();
        assert!(Arc::ptr_eq(&frozen, &reused));
        let mut observed = None;
        let mut result = advance_with_policy(
            Arc::clone(&current),
            &state.released,
            ONE_CANDIDATE_PER_BATCH,
            ReleasedSweepPolicy::Frozen,
            |candidate, budget| {
                let evaluation =
                    evaluate_global_candidate(&proof, candidate, budget, context, mode)?;
                observed = Some((candidate.live, evaluation));
                Ok(evaluation)
            },
        )
        .unwrap();
        let (evaluated, evaluation) = observed.unwrap();
        assert_eq!(evaluated, live);
        cache::retain(&mut result, &reused);
        if evaluation == CandidateEvaluation::Retry {
            let retained = result.advance.next.frozen_inventory().unwrap();
            assert!(Arc::ptr_eq(&frozen, &retained));
            state.commit(result.advance);
            assert!(Instant::now() < deadline, "{COMPLETION_TIMEOUT_DETAIL}");
            std::thread::yield_now();
            continue;
        }
        assert_eq!(evaluation, CandidateEvaluation::Complete(true));
        assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
        assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
        assert_eq!(
            result.candidates.observed,
            HashSet::from([live.lineage_id()])
        );
        assert!(result.advance.next.frozen_inventory().is_none());
        break;
    }
}
