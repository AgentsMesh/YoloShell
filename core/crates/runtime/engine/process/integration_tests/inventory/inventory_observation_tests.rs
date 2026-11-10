use super::PtyProcessScope;
use crate::process::controller_boundary::ProcessDiscoveryBoundary;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::collections::HashSet;

const REPLACEMENT_GENERATION_OFFSET: u64 = 1;

#[test]
fn root_exit_excludes_the_original_root_anchor() {
    let (scope, root) = scope_with_current_process();
    let boundary = ProcessDiscoveryBoundary::capture(&root, Vec::new()).unwrap();

    let requested = scope.requested_anchors(&HashSet::from([root]), &boundary, true);

    assert!(requested.is_empty());
}

#[test]
fn root_exit_retains_a_same_pid_replacement_generation() {
    let (scope, root) = scope_with_current_process();
    let boundary = ProcessDiscoveryBoundary::capture(&root, Vec::new()).unwrap();
    let replacement = replacement_identity(&root);
    let anchors = HashSet::from([root.clone(), replacement.clone()]);

    let requested = scope.requested_anchors(&anchors, &boundary, true);

    assert!(!requested.iter().any(|identity| identity == &root));
    assert!(requested.contains(&replacement));
}

fn scope_with_current_process() -> (PtyProcessScope, LiveProcessIdentity) {
    let durable = ProcessIdentity::capture(std::process::id()).unwrap();
    let scope = PtyProcessScope::capture(durable).unwrap();
    let root = scope.live_root();
    (scope, root)
}

fn replacement_identity(root: &LiveProcessIdentity) -> LiveProcessIdentity {
    let candidate = root.with_generation(root.lineage_id());
    if candidate != root.clone() {
        candidate
    } else {
        root.with_generation(
            root.lineage_id()
                .wrapping_add(REPLACEMENT_GENERATION_OFFSET),
        )
    }
}
