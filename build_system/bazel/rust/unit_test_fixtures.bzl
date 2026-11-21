LIVE_PEER_FIXTURE_TARGET = "//build_system/bazel/rust/fixtures:codex_live_peer_fixture"
VERSION_FIXTURE_TARGET = "//build_system/bazel/rust/fixtures:codex_version_fixture"
TEST_RUNTIME_TARGET = "//core:yolo-adversarial"

UNIT_TEST_FIXTURE_DEPS = [
    "//build_system/bazel/rust/fixtures:yolo_codex_live_peer_fixture_contract",
    "//build_system/bazel/rust/fixtures:yolo_codex_version_fixture_contract",
]

UNIT_TEST_FIXTURE_DATA = [
    LIVE_PEER_FIXTURE_TARGET,
    VERSION_FIXTURE_TARGET,
    TEST_RUNTIME_TARGET,
]

def unit_test_fixture_environment(test_threads):
    return {
        "RUST_TEST_THREADS": test_threads,
        "YOLO_CODEX_LIVE_PEER_FIXTURE": "$(rootpath {})".format(LIVE_PEER_FIXTURE_TARGET),
        "YOLO_CODEX_VERSION_FIXTURE": "$(rootpath {})".format(VERSION_FIXTURE_TARGET),
        "YOLO_TEST_RUNTIME": "$(rootpath {})".format(TEST_RUNTIME_TARGET),
    }
