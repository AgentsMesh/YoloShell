use super::test_support::{CHILD, MockApi, ROOT, entry};
use super::*;

#[test]
fn process_evidence_paths_are_injected_and_budgeted() {
    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let evidence = PathBuf::from("/workspace/session.jsonl");
    api.set_evidence_paths(vec![evidence.clone()]);
    let observed = descendants_with(&api, ROOT, ScanBudget::default()).unwrap();
    assert_eq!(observed[0].evidence_paths, Some(vec![evidence]));

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.set_evidence_paths(vec![PathBuf::from("/workspace/session.jsonl")]);
    let starved = ScanBudget::new(2, 10, 10, 1);
    assert!(descendants_with(&api, ROOT, starved).is_err());
}

#[test]
fn cwd_changes_during_observation_are_rejected() {
    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.set_cwds(vec![PathBuf::from("/first"), PathBuf::from("/second")]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn argv_read_failure_drops_the_incomplete_process_observation() {
    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.disable_argv_read();

    let observed = descendants_with(&api, ROOT, ScanBudget::default()).unwrap();

    assert!(observed.is_empty());
}
