use yolo_builtin_catalog::{TUI_ID, builtin_client_descriptors};
use yolo_builtin_client_contract::BuiltinClientDescriptor;
use yolo_plugin_api::Platform;

#[test]
fn bundled_tui_descriptor_is_valid() {
    let descriptors = builtin_client_descriptors();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].id(), TUI_ID);
    assert_eq!(descriptors[0].platform(), Platform::Unix);
    assert!(descriptors[0].entrypoint().arguments.is_empty());
}

#[test]
fn descriptor_constructor_rejects_empty_identity() {
    let descriptor = builtin_client_descriptors().remove(0);
    assert!(
        BuiltinClientDescriptor::new(
            "",
            descriptor.platform(),
            descriptor.entrypoint().clone(),
            descriptor.api().clone()
        )
        .is_err()
    );
}
