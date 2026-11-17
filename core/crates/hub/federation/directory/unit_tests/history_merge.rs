use crate::{history_support::*, support::*};

#[test]
fn disk_and_retired_generation_histories_merge_independently_of_restore_order() {
    let entry = Identity::new();
    let origin = Identity::new();
    let disk_only = Identity::new();
    let retired_only = Identity::new();
    let old = catalogue(&origin);
    let mut new = old.payload.clone();
    new.revision += STEP;
    new.policy_epoch += STEP;
    let newest = resign(new, &origin);
    let disk = history(entry.identity, vec![old, catalogue(&disk_only)]);
    let retired = history(
        entry.identity,
        vec![newest.clone(), catalogue(&retired_only)],
    );
    let mut disk_first = offline(entry.identity);
    let mut retired_first = offline(entry.identity);
    disk_first.restore_history(disk.clone()).unwrap();
    disk_first.restore_history(retired.clone()).unwrap();
    retired_first.restore_history(retired).unwrap();
    retired_first.restore_history(disk).unwrap();
    assert_eq!(disk_first.history(), retired_first.history());
    assert_eq!(
        disk_first.latest_for(origin.identity.hub()).as_deref(),
        Some(&newest)
    );
    for known in [&origin, &disk_only, &retired_only] {
        assert!(disk_first.latest_for(known.identity.hub()).is_some());
        assert!(
            disk_first
                .route_for(known.identity.hub(), &observe(), NOW)
                .unwrap()
                .is_none()
        );
    }
    assert!(disk_first.exportable(NOW).unwrap().is_empty());
}

#[test]
fn repeated_and_empty_history_restores_preserve_an_existing_observation() {
    let entry = Identity::new();
    let origin = Identity::new();
    let retained = history(entry.identity, vec![catalogue(&origin)]);
    let mut book = offline(entry.identity);
    book.restore_history(retained.clone()).unwrap();
    book.restore_history(retained.clone()).unwrap();
    book.restore_history(history(entry.identity, Vec::new()))
        .unwrap();
    assert_eq!(book.history(), retained);
}

#[test]
fn older_revision_and_newer_revision_with_lower_policy_cannot_overwrite_history() {
    let entry = Identity::new();
    let origin = Identity::new();
    let mut current = catalogue(&origin).payload;
    current.revision += STEP;
    current.policy_epoch += STEP;
    let retained = history(entry.identity, vec![resign(current.clone(), &origin)]);
    let mut old_revision = current.clone();
    old_revision.revision -= STEP;
    old_revision.policy_epoch += STEP;
    let mut old_policy = current;
    old_policy.revision += STEP;
    old_policy.policy_epoch -= STEP;
    let mut book = offline(entry.identity);
    book.restore_history(retained.clone()).unwrap();
    for payload in [old_revision, old_policy] {
        book.restore_history(history(entry.identity, vec![resign(payload, &origin)]))
            .unwrap();
        assert_eq!(book.history(), retained);
    }
}

#[test]
fn newer_revision_with_equal_policy_replaces_the_observation_without_creating_authority() {
    let entry = Identity::new();
    let origin = Identity::new();
    let before = catalogue(&origin);
    let mut payload = before.payload.clone();
    payload.revision += STEP;
    let after = resign(payload, &origin);
    let mut book = offline(entry.identity);
    book.restore_history(history(entry.identity, vec![before]))
        .unwrap();
    book.restore_history(history(entry.identity, vec![after.clone()]))
        .unwrap();
    assert_eq!(
        book.latest_for(origin.identity.hub()).as_deref(),
        Some(&after)
    );
    assert!(book.allowed_origins().is_empty());
    assert!(
        book.route_for(origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_none()
    );
    assert!(book.exportable(NOW).unwrap().is_empty());
}
