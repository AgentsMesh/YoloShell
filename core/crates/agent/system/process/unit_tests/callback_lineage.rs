use super::super::fallback_lineage::is_foreground_candidate;
use super::{fixture::*, *};

#[test]
fn callback_own_process_group_is_visible_without_widening_foreground_discovery() {
    let api = Api::new();
    let observed = observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).unwrap();
    assert_eq!(
        observed
            .iter()
            .map(|process| process.pid)
            .collect::<Vec<_>>(),
        [PEER, LAUNCHER, PROVIDER]
    );
    assert_eq!(observed[0].process_group, i32::try_from(LAUNCHER).unwrap());
    assert_eq!(
        observed[0].foreground_group,
        i32::try_from(PROVIDER).unwrap()
    );
    let process = &observed[0];
    let row = ProcessRow {
        pid: process.pid,
        parent_pid: process.parent_pid,
        process_group: process.process_group,
        foreground_group: process.foreground_group,
        executable: process.executable.clone(),
        inventory_identity: None,
    };
    assert!(!is_foreground_candidate(
        &row,
        &api.identities.borrow()[&ROOT]
    ));
}

#[test]
fn incomplete_cyclic_foreign_terminal_and_root_only_chains_are_rejected() {
    for change in [
        (|api: &Api| {
            api.identities.borrow_mut().remove(&LAUNCHER);
        }) as fn(&Api),
        |api: &Api| {
            api.identities
                .borrow_mut()
                .get_mut(&LAUNCHER)
                .unwrap()
                .parent_pid = PEER;
        },
        |api: &Api| {
            api.identities
                .borrow_mut()
                .get_mut(&PEER)
                .unwrap()
                .foreground_group = NO_FOREGROUND;
        },
        |api: &Api| {
            api.identities
                .borrow_mut()
                .get_mut(&ROOT)
                .unwrap()
                .foreground_group = NO_FOREGROUND;
        },
        |api: &Api| {
            api.identities.borrow_mut().get_mut(&PEER).unwrap().pid = UNRELATED;
        },
    ] {
        let api = Api::new();
        change(&api);
        assert!(observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).is_none());
    }
    assert!(observe_lineage(&Api::new(), ROOT, ROOT, &[ENVIRONMENT_KEY]).is_none());
    assert!(observe_lineage(&Api::new(), ROOT, UNRELATED, &[ENVIRONMENT_KEY]).is_none());
}

#[test]
fn stable_peer_cannot_hide_reparented_reused_or_exec_changed_ancestors() {
    for (pid, change) in [
        (
            LAUNCHER,
            (|identity: &mut ProcessIdentity| {
                identity.parent_pid = UNRELATED;
            }) as fn(&mut ProcessIdentity),
        ),
        (LAUNCHER, |identity: &mut ProcessIdentity| {
            identity.start_time += STEP;
        }),
        (LAUNCHER, |identity: &mut ProcessIdentity| {
            identity.executable_file.inode += STEP;
        }),
        (LAUNCHER, |identity: &mut ProcessIdentity| {
            identity.cwd.path = REPLACEMENT_CWD.into();
        }),
        (ROOT, |identity: &mut ProcessIdentity| {
            identity.process_inode += STEP;
        }),
        (PEER, |identity: &mut ProcessIdentity| {
            identity.process_group += i32::try_from(STEP).unwrap();
        }),
    ] {
        let api = Api::new();
        api.mutate_during_observation(pid, change);
        assert!(observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).is_none());
    }
}

#[test]
fn a_chain_wide_second_observation_rejects_changed_earlier_environment() {
    let api = Api::new();
    api.change_peer_after_provider.set(true);
    assert!(observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).is_none());
    let api = Api::new();
    api.unavailable.set(true);
    assert!(observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).is_none());
}

#[test]
fn ancestry_walk_is_bounded_before_a_missing_or_reparented_root_can_be_reached() {
    let api = Api::new();
    let mut identities = api.identities.borrow_mut();
    for offset in 0..MAX_PROCESS_ANCESTOR_DEPTH {
        let pid = DEPTH_PID_BASE + u32::try_from(offset).unwrap();
        identities.insert(
            pid,
            identity(pid, pid + u32::try_from(STEP).unwrap(), LAUNCHER),
        );
    }
    drop(identities);
    assert!(observe_lineage(&api, ROOT, DEPTH_PID_BASE, &[ENVIRONMENT_KEY]).is_none());
    assert!(
        api.identity_reads.get() <= MAX_PROCESS_ANCESTOR_DEPTH + usize::try_from(STEP).unwrap()
    );
}

fn shell() -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: ROOT,
        start_time: START_TIME,
    }
}

#[test]
fn ancestry_accepts_a_callback_in_its_own_terminal_session() {
    let api = Api::new();
    {
        let mut identities = api.identities.borrow_mut();
        let peer = identities.get_mut(&PEER).unwrap();
        peer.process_group = i32::try_from(PEER).unwrap();
        peer.foreground_group = i32::try_from(PEER).unwrap();
    }
    let observed = observe_ancestry(&api, shell(), PEER, &[ENVIRONMENT_KEY]).unwrap();
    assert_eq!(
        observed.iter().map(|process| process.pid).collect::<Vec<_>>(),
        [PEER, LAUNCHER, PROVIDER]
    );
    // The foreground-fenced variant still rejects the detached callback.
    assert!(observe_lineage(&api, ROOT, PEER, &[ENVIRONMENT_KEY]).is_none());
}

#[test]
fn ancestry_requires_the_exact_custody_shell() {
    let api = Api::new();
    api.identities
        .borrow_mut()
        .get_mut(&PROVIDER)
        .unwrap()
        .parent_pid = UNRELATED;
    assert!(observe_ancestry(&api, shell(), PEER, &[ENVIRONMENT_KEY]).is_none());

    for (root, peer) in [
        (
            crate::process::ProcessIdentity {
                pid: ROOT,
                start_time: START_TIME + STEP,
            },
            PEER,
        ),
        (shell(), ROOT),
        (shell(), UNRELATED),
    ] {
        assert!(observe_ancestry(&Api::new(), root, peer, &[ENVIRONMENT_KEY]).is_none());
    }
}
