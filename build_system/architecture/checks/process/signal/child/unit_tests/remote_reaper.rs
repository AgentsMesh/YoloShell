use super::support::reject_mutations;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn reservation_retains_exact_child_retry_state_before_disarming_and_notifying() {
    reject_mutations(
        contract::BROKER_REAPER_CUSTODY_PATH,
        &[
            ("if termination_required {", "if unrelated {"),
            ("queue.termination_count +=", "queue.custody_count +="),
            ("queue.children.push_back(", "queue.children.discard("),
            (
                "self.shared.ready.notify_one();",
                "self.shared.ready.notify_all();",
            ),
            (
                "self.shared.ready.notify_one();",
                "self.shared.ready.notify_one(); self.shared.ready.notify_one();",
            ),
            (
                "            termination_required,",
                "            unrelated,",
            ),
            (
                "        self.active = false;",
                "        self.active = true;",
            ),
            ("if self.active {", "if unrelated {"),
            (".custody_count -=", ".termination_count -="),
        ],
    );
}

#[test]
fn worker_panic_fallback_and_queue_transfer_keep_the_retained_children() {
    reject_mutations(
        contract::BROKER_REAPER_WORKER_PATH,
        &[
            (
                "let result = containment::catch_unwind(",
                "let result = other::catch_unwind(",
            ),
            (
                "let _ = containment::catch_unwind(",
                "let _ = other::catch_unwind(",
            ),
            (
                "queue.worker_failed = true;",
                "queue.worker_failed = false;",
            ),
            (
                "fallback(&shared, &mut retained);",
                "discard(&shared, &mut retained);",
            ),
            ("receive(shared, retained);", "discard(shared, retained);"),
            (
                "let progress = advance(retained);",
                "let progress = discard(retained);",
            ),
            (
                "retained.extend(queue.children.drain(..));",
                "discard(queue.children.drain(..));",
            ),
            ("std::thread::sleep(", "std::thread::yield_now("),
            (
                "self.try_wait().map(|status| status.is_some())",
                "self.try_wait().ok().map(|status| status.is_some())",
            ),
            (
                "self.try_wait().map(|status| status.is_some())",
                "self.try_wait().map(|status| status.is_none())",
            ),
        ],
    );
}

#[test]
fn retained_reaper_removes_only_waited_children_and_preserves_retry_accounting() {
    reject_mutations(
        contract::BROKER_REAPER_WORKER_PATH,
        &[
            (
                "match child.child.try_reap()",
                "match child.child.poll_untrusted()",
            ),
            (
                "match child.child.terminate()",
                "match child.child.abandon()",
            ),
            ("children.swap_remove(index);", "discard(index);"),
            ("is_child_already_reaped(&error)", "is_unrelated(&error)"),
            (
                "if child.termination_required {\n            match",
                "if unrelated {\n            match",
            ),
            ("process_is_gone(&error)", "is_unrelated(&error)"),
            (
                "child.termination_required = false;",
                "child.termination_required = true;",
            ),
            ("progress.terminations +=", "progress.unrelated +="),
            ("progress.settled +=", "progress.unrelated +="),
            ("index +=", "index ="),
        ],
    );
}
