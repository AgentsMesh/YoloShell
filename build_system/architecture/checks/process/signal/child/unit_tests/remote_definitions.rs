use super::constants::REPLACED_FUNCTION;
use super::support::{assert_flow, source};
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn every_remote_lifecycle_entrypoint_is_required_and_unambiguous() {
    for (path, names) in [
        (
            contract::BROKER_PENDING_PATH,
            &[
                "compensate",
                "drop",
                "handoff",
                contract::COMPENSATE_HANDOFF_FAILURE,
            ][..],
        ),
        (
            contract::BROKER_SETTLEMENT_PATH,
            &[
                contract::SETTLE_CHILD,
                contract::SETTLE_CHILD_ON_DROP,
                contract::SETTLEMENT_ACTIONS,
                contract::RETAIN_UNSETTLED,
                contract::RETAIN_FOR_DROP,
            ][..],
        ),
        (
            contract::BROKER_REAPER_CUSTODY_PATH,
            &[contract::RETAIN, contract::DROP][..],
        ),
        (
            contract::BROKER_REAPER_WORKER_PATH,
            &[
                contract::RUN,
                contract::FALLBACK,
                contract::RUN_ITERATION,
                "receive",
                contract::ADVANCE,
                contract::TRY_REAP,
            ][..],
        ),
    ] {
        let baseline = source(path);
        assert_flow(path, &baseline, true);
        assert_flow(path, "", false);
        for name in names {
            let needle = format!("fn {name}");
            let renamed = baseline.replace(&needle, &format!("fn {REPLACED_FUNCTION}"));
            assert_ne!(renamed, baseline, "{path}: {name}");
            assert_flow(path, &renamed, false);
        }
        assert_flow(path, &format!("{baseline}\n{baseline}"), false);
    }
}
