use super::entrypoint_fixture as fixture;
use super::support::{inspect, messages, source};

#[test]
fn schema_macro_review_has_exact_origins_and_cannot_hide_binary_or_unknown_actions() {
    use crate::constants as contract;
    let call = "load(\"//build_system/bazel/capnp:defs.bzl\", \"capnp_rust_library\")\ncapnp_rust_library(name=\"schema\",srcs=[])";
    assert!(inspect(fixture::schema_files(call)).is_empty());
    for mutation in [
        "rust_binary(name=\"hidden\",srcs=[])\n",
        "ctx.actions.unreviewed()\n",
    ] {
        let mut changed = fixture::schema_files(call);
        changed.last_mut().unwrap().text.push_str(mutation);
        assert!(messages(&inspect(changed)).contains(contract::EXECUTABLE_POLICY));
    }
    let mut changed = fixture::schema_files(call);
    changed
        .iter_mut()
        .find(|file| file.relative == fixture::OTHER_BUILD)
        .unwrap()
        .text = call.replace("//build_system/bazel/capnp:defs.bzl", "//unreviewed:defs.bzl");
    assert!(messages(&inspect(changed)).contains(contract::CALLABLE_POLICY));
}

#[test]
fn static_library_and_native_header_wrapper_still_reject_untrusted_rule_origins() {
    use crate::constants as contract;
    let bridge = "load(\"@rules_rust//rust:defs.bzl\", \"rust_static_library\")\nrust_static_library(name=\"bridge\",srcs=[])\ncc_library(name=\"headers\",deps=[\":bridge\"])";
    let mut files = fixture::files(fixture::ENTRY);
    files.push(source(fixture::OTHER_BUILD, bridge));
    assert!(inspect(files).is_empty());
    let mut files = fixture::files(fixture::ENTRY);
    files.push(source(fixture::OTHER_BUILD, bridge));
    files.last_mut().unwrap().text =
        bridge.replace("@rules_rust//rust:defs.bzl", "//unreviewed:defs.bzl");
    assert!(messages(&inspect(files)).contains(contract::RULE_RESOLUTION_POLICY));
}
