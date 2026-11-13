use yolo_tui_descriptor::{TUI_ID, tui_descriptor};
use yolo_builtin_client_contract::BuiltinClientDescriptor;
use yolo_plugin_api::Platform;

#[test]
fn bundled_tui_descriptor_is_valid() {
    let descriptor = tui_descriptor();
    assert_eq!(descriptor.id(), TUI_ID);
    assert_eq!(descriptor.platform(), Platform::Unix);
    assert!(descriptor.entrypoint().arguments.is_empty());
}

#[test]
fn descriptor_constructor_rejects_empty_identity() {
    let descriptor = tui_descriptor();
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
