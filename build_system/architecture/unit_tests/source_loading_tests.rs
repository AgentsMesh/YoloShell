use super::*;
use std::os::unix::fs::symlink;
use yolo_architecture_test_support::Fixture;

#[test]
fn source_loading_is_sorted_and_preserves_extensionless_shells() {
    let fixture = Fixture::new();
    fixture.write("zeta.rs", "fn last() {}\n");
    fixture.write("nested/alpha.md", "# First\n");
    fixture.write("runner", "#!/bin/sh\nexit 0\n");
    fixture.write("opaque", "ordinary text\n");
    fixture.write("image.png", [u8::MAX]);
    let files = load(fixture.root()).unwrap();
    assert_eq!(
        files
            .iter()
            .map(|file| file.relative.as_str())
            .collect::<Vec<_>>(),
        ["nested/alpha.md", "runner", "zeta.rs"]
    );
    assert_eq!(files[0].absolute, fixture.root().join("nested/alpha.md"));
    assert_eq!(files[0].text, "# First\n");
    assert!(!scanned(Path::new("")));
    assert!(!extensionless_shebang(&fixture.root().join("zeta.rs")));
    assert!(!extensionless_shebang(fixture.root()));
}

#[test]
fn capnp_schema_inventory_includes_handwritten_inputs_but_excludes_generated_output_roots() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/transport/capnp/schema/rpc.capnp",
        "@0xf000000000000001;\n",
    );
    fixture.write(".build/generated/rpc_capnp.rs", [u8::MAX]);
    fixture.write("bazel-out/generated/rpc.capnp", [u8::MAX]);
    let files = load(fixture.root()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].relative,
        "core/crates/transport/capnp/schema/rpc.capnp"
    );
}

#[test]
fn native_bridge_headers_and_module_maps_are_visible_to_architecture_checks() {
    let fixture = Fixture::new();
    fixture.write("bridge/include/value.h", "#include <stdint.h>\n");
    fixture.write("bridge/include/module.modulemap", "module Bridge {}\n");
    assert_eq!(
        load(fixture.root())
            .unwrap()
            .iter()
            .map(|file| file.relative.as_str())
            .collect::<Vec<_>>(),
        ["bridge/include/module.modulemap", "bridge/include/value.h"]
    );
}

#[test]
fn root_generated_directories_do_not_hide_nested_real_sources() {
    let fixture = Fixture::new();
    fixture.write(".build/invalid.rs", [u8::MAX]);
    fixture.write("bazel-custom/invalid.rs", [u8::MAX]);
    fixture.write("nested/.build/kept.rs", "fn kept() {}\n");
    let files = load(fixture.root()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative, "nested/.build/kept.rs");
    assert!(ignored(fixture.root(), fixture.root()));
    assert!(!root_entry(fixture.root(), Path::new("/outside")));
    assert!(!root_entry(fixture.root(), fixture.root()));
    assert!(!root_entry(
        fixture.root(),
        &fixture.root().join("nested/file")
    ));
    assert_eq!(relative(fixture.root(), Path::new("/outside")), "/outside");
}

#[test]
fn dangling_links_and_nonregular_devices_are_not_read_as_source_files() {
    let fixture = Fixture::new();
    symlink(
        fixture.root().join("absent"),
        fixture.root().join("dangling.rs"),
    )
    .unwrap();
    symlink("/dev/null", fixture.root().join("device.rs")).unwrap();
    assert!(load(fixture.root()).unwrap().is_empty());
    assert!(directories(fixture.root()).unwrap().is_empty());
}

#[test]
fn ancestor_directory_links_terminate_without_dropping_real_sources() {
    let fixture = Fixture::new();
    fixture.write("nested/kept.rs", "fn kept() {}\n");
    symlink(fixture.root(), fixture.root().join("nested/back")).unwrap();
    let files = load(fixture.root()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative, "nested/kept.rs");
    assert_eq!(
        directories(fixture.root()).unwrap(),
        ["nested", "nested/back"]
    );
}

#[test]
fn missing_roots_invalid_text_and_unresolvable_links_are_errors() {
    let fixture = Fixture::new();
    assert!(load(&fixture.root().join("absent")).is_err());
    assert!(directories(&fixture.root().join("absent")).is_err());
    fixture.write("invalid.rs", [u8::MAX]);
    assert_eq!(
        load(fixture.root()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let cycles = Fixture::new();
    symlink("right", cycles.root().join("left")).unwrap();
    symlink("left", cycles.root().join("right")).unwrap();
    assert!(load(cycles.root()).is_err());
    assert!(directories(cycles.root()).is_err());
}
