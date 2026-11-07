use yolo_architecture_test_support::Fixture;

const ADAPTER: &str = "core/crates/application/local/profile/src/foundation.rs";
const OTHER_COMPONENT: &str = "core/crates/runtime/engine/worker/src/platform.rs";
const BINDING: &str = include_str!("../src/foundation_directory.fixture");

fn inspect(path: &str, source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(path, source);
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn only_the_named_filesystem_adapter_accepts_its_complete_native_binding() {
    assert!(inspect(ADAPTER, BINDING).is_empty());
    assert!(!inspect(OTHER_COMPONENT, BINDING).is_empty());
    assert!(!inspect("Cargo.toml", "objc = \"1\"").is_empty());
}

#[test]
fn comments_and_whitespace_preserve_the_reviewed_binding() {
    let source = BINDING
        .replace(
            "unsafe extern",
            "unsafe /* binding /* nested */ comment */\nextern",
        )
        .replace("fn object", "fn\n\tobject")
        .replace(";", ";\n\n");
    assert!(inspect(ADAPTER, &source).is_empty());
    assert!(inspect(ADAPTER, &format!("// Directory resolver\n{BINDING}")).is_empty());
}

#[test]
fn filesystem_adapter_cannot_add_classes_selectors_frameworks_or_general_runtime_apis() {
    for source in [
        BINDING.replace("NSFileManager", "NSWindow"),
        BINDING.replace("fileSystemRepresentation", "makeKeyAndOrderFront:"),
        BINDING.replace("Foundation", "AppKit"),
        BINDING.replace("Foundation", "Cocoa"),
        format!("{BINDING}\nunsafe extern \"C\" {{ fn objc_getClass(); }}"),
        format!(
            "{BINDING}\nunsafe extern \"C\" {{ fn object_getClass(value: Object) -> Object; }}"
        ),
        format!(
            "{BINDING}\nunsafe extern \"C\" {{ fn sel_getName(value: Selector) -> *const c_char; }}"
        ),
        format!("{BINDING}\nuse objc::runtime::Class;"),
        format!("{BINDING}\nuse objc/**/::runtime::Class;"),
        format!("{BINDING}\nextern crate objc as runtime;"),
    ] {
        assert!(!inspect(ADAPTER, &source).is_empty(), "{source}");
    }
}

#[test]
fn private_runtime_helpers_cannot_be_exported_or_wrapped_as_general_bridges() {
    let wrapper = r#"
pub(super) unsafe fn send_any(receiver: Object, selector: &CStr) -> Result<Object> {
    unsafe { object(receiver, selector) }
}
"#;
    for source in [
        format!("{BINDING}{wrapper}"),
        BINDING.replace("unsafe fn object", "pub(super) unsafe fn object"),
        BINDING.replace("fn nonnull", "pub fn nonnull"),
    ] {
        assert!(!inspect(ADAPTER, &source).is_empty(), "{source}");
    }
}

#[test]
fn comments_cannot_forge_a_complete_binding_or_hide_selector_changes() {
    for source in [
        format!("/*{BINDING}*/\nuse objc/**/::runtime::Class;"),
        BINDING.replace(
            "c\"firstObject\"",
            "/* c\"firstObject\" */ c\"description\"",
        ),
        format!("{BINDING}\nconst EXTRA: &CStr = c\"description\\\n\";"),
        BINDING.replace(
            "object(urls, FIRST_OBJECT)",
            "object(urls, DEFAULT_MANAGER)",
        ),
    ] {
        assert!(!inspect(ADAPTER, &source).is_empty(), "{source}");
    }
}

#[test]
fn partial_binding_and_ui_dependencies_remain_rejected() {
    let partial = BINDING.replace("c\"firstObject\"", "c\"\"");
    assert!(!inspect(ADAPTER, &partial).is_empty());
    for source in ["use objc::runtime::Class;", "use cocoa::appkit::NSWindow;"] {
        assert!(!inspect(ADAPTER, source).is_empty());
        assert!(!inspect(OTHER_COMPONENT, source).is_empty());
    }
}

#[test]
fn rust_ui_detection_uses_identifier_boundaries_and_keeps_real_dependencies_blocked() {
    assert!(
        inspect(
            OTHER_COMPONENT,
            "fn sliced_network_buffers() {}\nfn entauriated() {}\n"
        )
        .is_empty()
    );
    for dependency in [
        "use iced::Settings;",
        "extern crate iced;",
        "use ICED::Settings;",
    ] {
        assert!(
            !inspect(OTHER_COMPONENT, dependency).is_empty(),
            "{dependency}"
        );
    }
    assert!(!inspect("Cargo.toml", "iced = \"0.1\"").is_empty());
}
