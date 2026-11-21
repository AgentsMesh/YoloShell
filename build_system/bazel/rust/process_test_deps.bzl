
load(
    "//build_system/architecture:topology.bzl",
    "YOLO_APPLICATION_SUPPORT",
    "YOLO_APPLICATION_SUPPORT_TEST_SUPPORT",
    "YOLO_LOCAL_UNIX_TEST_SUPPORT",
    "YOLO_PATHS",
    "YOLO_PTY_CUSTODY",
)
PROCESS_COMPONENT_DEPS = [
    YOLO_APPLICATION_SUPPORT,
    "//core/crates/process/spawn:yolo_process_spawn",
    "//core/crates/runtime/engine/process/identity:yolo_process_identity",
    YOLO_PTY_CUSTODY,
]

PROCESS_TEST_ALIASES = {
    YOLO_APPLICATION_SUPPORT_TEST_SUPPORT: "yolo_application_support",
    "//core/crates/process/spawn:yolo_process_spawn_test_support": "yolo_process_spawn",
    "//core/crates/runtime/engine/process/identity:yolo_process_identity_test_support": "yolo_process_identity",
    "//core/crates/runtime/engine/pty/custody:yolo_pty_custody_test_support": "yolo_pty_custody",
    YOLO_LOCAL_UNIX_TEST_SUPPORT: "yolo_local_unix",
}

def process_test_deps(common_deps):
    return common_deps + PROCESS_TEST_ALIASES.keys() + [
        YOLO_PATHS,
        "@crates//:serde_json",
    ]
