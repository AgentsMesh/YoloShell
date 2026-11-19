use super::test_support::{CHILD, GROUP, MockApi, ROOT, entry};
use super::*;

#[test]
fn idle_shell_stops_before_expensive_process_reads() {
    let api = MockApi::new([entry(ROOT, 1)], vec![ROOT]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
    assert!(api.expensive.borrow().is_empty());
}

#[test]
fn foreground_descendant_is_observed_with_structured_fields() {
    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![ROOT, CHILD]);
    let observed = descendants_with(&api, ROOT, ScanBudget::default()).unwrap();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].pid, CHILD);
    assert_eq!(observed[0].argv, ["codex"]);
    assert_eq!(
        observed[0].cwd.as_deref(),
        Some(std::path::Path::new("/workspace"))
    );
}

#[test]
fn unrelated_and_racing_group_members_are_rejected() {
    let unrelated = entry(CHILD, 99);
    let api = MockApi::new([entry(ROOT, 1), unrelated], vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
    assert!(api.expensive.borrow().is_empty());

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.push_metadata(CHILD, entry(CHILD, 99));
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let mut reused = entry(CHILD, ROOT);
    reused.start_time = reused.start_time.wrapping_add(1);
    api.push_metadata(CHILD, reused);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.set_executables(vec![
        PathBuf::from("/usr/bin/codex"),
        PathBuf::from("/bin/sh"),
    ]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let mut changed_shell = entry(ROOT, 1);
    changed_shell.start_time = changed_shell.start_time.wrapping_add(1);
    api.push_metadata(ROOT, changed_shell);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn adversarial_inventory_and_budgets_fail_closed() {
    let api = MockApi::new([entry(ROOT, 1)], vec![ROOT, CHILD]);
    let budget = ScanBudget::new(1, 10, 10, 10);
    assert!(descendants_with(&api, ROOT, budget).is_err());

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let budget = ScanBudget::new(2, 0, 10, 10);
    assert!(descendants_with(&api, ROOT, budget).is_err());

    let mut invalid_shell = entry(ROOT, 1);
    invalid_shell.foreground_group = 0;
    let api = MockApi::new([invalid_shell], Vec::new());
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn cycles_wrong_groups_and_resource_exhaustion_never_observe_candidates() {
    let mut wrong_group = entry(CHILD, ROOT);
    wrong_group.process_group = GROUP + 1;
    let api = MockApi::new([entry(ROOT, 1), wrong_group], vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
    assert!(api.expensive.borrow().is_empty());

    let mut wrong_foreground = entry(CHILD, ROOT);
    wrong_foreground.foreground_group = GROUP + 1;
    let api = MockApi::new([entry(ROOT, 1), wrong_foreground], vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let next = CHILD + 1;
    let api = MockApi::new(
        [entry(ROOT, 1), entry(CHILD, next), entry(next, CHILD)],
        vec![CHILD, next],
    );
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );
    assert!(api.expensive.borrow().is_empty());

    let api = MockApi::new(
        [entry(ROOT, 1), entry(CHILD, next), entry(next, ROOT)],
        vec![CHILD],
    );
    assert_eq!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .len(),
        1
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let path_starved = ScanBudget::new(2, 10, 10, 1);
    assert!(descendants_with(&api, ROOT, path_starved).is_err());

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    let argv_starved = ScanBudget::new(2, 10, 1, 100);
    assert!(descendants_with(&api, ROOT, argv_starved).is_err());

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, 0)], vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.set_executables(vec![PathBuf::new()]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.set_executables(vec![PathBuf::from("/usr/bin/codex"), PathBuf::new()]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, CHILD)], vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let mut deep = vec![entry(ROOT, 1)];
    deep.extend((0..=MAX_PROCESS_ANCESTOR_DEPTH as u32).map(|offset| {
        let pid = CHILD + offset;
        entry(pid, pid + 1)
    }));
    let api = MockApi::new(deep, vec![CHILD]);
    assert!(
        descendants_with(&api, ROOT, ScanBudget::default())
            .unwrap()
            .is_empty()
    );

    let api = MockApi::new([entry(ROOT, 1), entry(CHILD, ROOT)], vec![CHILD]);
    api.disable_optional_reads();
    let observed = descendants_with(&api, ROOT, ScanBudget::default()).unwrap();
    assert_eq!(observed[0].argv, ["codex"]);
    assert!(observed[0].cwd.is_none());
    assert!(observed[0].evidence_paths.is_none());
}
