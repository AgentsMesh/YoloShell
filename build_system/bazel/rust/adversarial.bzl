load("@rules_rust//rust:defs.bzl", "rust_library")
load("//build_system/bazel:version.bzl", "YOLO_MARKETING_VERSION")
load(
    ":component_deps.bzl",
    "RUNTIME_ADVERSARIAL_COMPONENT_ALIASES",
    "RUNTIME_ADVERSARIAL_COMPONENT_DEPS",
)
load(":config.bzl", "RUNTIME_DEPS")

ADVERSARIAL_TEST_FEATURES = ["adversarial-test-hooks"]

def adversarial_runtime_targets():
    rust_library(
        name = "yolo_runtime_adversarial",
        testonly = True,
        srcs = ["//core:runtime_facade_sources"],
        crate_features = ADVERSARIAL_TEST_FEATURES,
        aliases = RUNTIME_ADVERSARIAL_COMPONENT_ALIASES,
        crate_name = "yolo_runtime",
        crate_root = "//core:src/lib.rs",
        compile_data = [
            "//core:lifecycle_deadline_contract",
            "//core:scm_rights_descriptor_contract",
        ],
        edition = "2024",
        version = YOLO_MARKETING_VERSION,
        deps = RUNTIME_DEPS + RUNTIME_ADVERSARIAL_COMPONENT_DEPS,
    )
