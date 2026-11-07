use super::fixture::*;
use crate::admission::select;
use crate::parent::candidate_from;
use crate::HookProcessIdentity;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use yolo_process::ProcessIdentity;

#[test]
fn parent_candidate_skips_callback_shells_to_the_agent() {
    let parents = BTreeMap::from([
        (10, (20, HOOK.to_owned())),
        (20, (30, BASH.to_owned())),
        (30, (40, NODE.to_owned())),
        (40, (1, "/sbin/init".to_owned())),
    ]);
    let candidate = candidate_from(10, 8, |pid| {
        let (parent_pid, executable) = parents[&pid].clone();
        Ok(crate::parent::Parent {
            pid,
            parent_pid,
            start_time: u64::from(pid),
            executable: PathBuf::from(executable),
        })
    })
    .unwrap();
    assert_eq!(
        candidate,
        HookProcessIdentity {
            pid: 30,
            start_time: 30
        }
    );
}

#[test]
fn admission_stops_at_the_claimed_agent_above_a_launcher() {
    let agent = HookProcessIdentity {
        pid: 30,
        start_time: 30,
    };
    let scope = hook_scope(agent);
    let processes = vec![
        process(10, 20, HOOK),
        process(20, 30, BASH),
        agent_process(30, 40, scope),
        process(40, 50, NODE),
        process(50, 1, BASH),
    ];
    let admitted = select(
        10,
        &profile(),
        ProcessIdentity {
            pid: 50,
            start_time: 50,
        },
        scope,
        Path::new(HOOK),
        &processes,
        &NodeAgent,
    )
    .unwrap();
    assert_eq!(admitted.agent.pid, 30);
}

#[test]
fn admission_rejects_a_non_shell_intermediate_or_a_foreign_agent() {
    let agent = HookProcessIdentity {
        pid: 30,
        start_time: 30,
    };
    let scope = hook_scope(agent);
    let shell = ProcessIdentity {
        pid: 50,
        start_time: 50,
    };
    let admit = |scope, processes| {
        select(
            10,
            &profile(),
            shell,
            scope,
            Path::new(HOOK),
            processes,
            &NodeAgent,
        )
    };
    let intermediate = vec![
        process(10, 20, HOOK),
        process(20, 30, BASH),
        process(30, 50, "/usr/bin/python"),
        process(50, 1, BASH),
    ];
    assert!(admit(scope, &intermediate).is_err());

    let claimed = HookProcessIdentity {
        pid: 40,
        start_time: 40,
    };
    let mismatched = vec![
        process(10, 20, HOOK),
        process(20, 30, BASH),
        agent_process(30, 50, scope),
        process(50, 1, BASH),
    ];
    assert!(admit(hook_scope(claimed), &mismatched).is_err());

    let mut foreign = agent_process(30, 50, scope);
    foreign.environment = Some(BTreeMap::from([(
        yolo_agent_core::constants::YOLO_TERMINAL_ID_ENV.into(),
        "other".into(),
    )]));
    let foreign = vec![
        process(10, 20, HOOK),
        process(20, 30, BASH),
        foreign,
        process(50, 1, BASH),
    ];
    assert!(admit(scope, &foreign).is_err());
}
