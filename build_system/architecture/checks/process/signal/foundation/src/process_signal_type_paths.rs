use crate::process_signal_guardian_constants as guardian;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;

pub const TYPE_PATHS: &[(&str, &[&str])] = &[
    (
        contract::RETAINED_EXACT_PROCESS,
        &[
            contract::EXACT_TERMINATION_PATH,
            contract::RETAINED_EXACT_DOUBLE_FORK_TESTS_PATH,
            "core/crates/runtime/engine/process/src/mod.rs",
            contract::REMOTE_SUPERVISED_CONTAINMENT_PATH,
            guardian::PATH,
        ],
    ),
    (
        contract::PROCESS_SIGNAL_AUTHORITY,
        &[
            contract::SIGNAL_AUTHORITY_PATH,
            contract::SIGNAL_AUTHORITY_EXACT_PATH,
            contract::LIVE_CONTROL_PATH,
            contract::PIDFD_PATH,
            contract::MACOS_SIGNAL_PATH,
        ],
    ),
    (
        contract::DESTRUCTIVE_AUTHORITY,
        &[
            contract::SIGNAL_AUTHORITY_PATH,
            contract::SIGNAL_AUTHORITY_EXACT_PATH,
            contract::CAPTURED_TARGETS_PATH,
            contract::CAPTURED_TARGETS_SIGKILL_PATH,
            contract::CAPTURED_TREE_PATH,
        ],
    ),
    (
        contract::FOREGROUND_AUTHORITY,
        &[contract::SIGNAL_AUTHORITY_PATH, contract::INVENTORY_PATH],
    ),
    (
        contract::PROCESS_GROUP_IDENTITY,
        &[
            contract::INVENTORY_PATH,
            contract::INVENTORY_SCOPE_PATH,
            contract::INVENTORY_MEMBER_SIGNAL_PATH,
            retained::PROCESS_MODULE_PATH,
            "core/crates/runtime/engine/process/integration_tests/inventory/inventory_tests.rs",
        ],
    ),
];
