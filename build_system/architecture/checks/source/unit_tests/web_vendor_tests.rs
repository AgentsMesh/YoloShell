use super::*;

#[test]
fn vendored_web_assets_are_limited_to_the_exact_allowlist() {
    for path in [
        "plugins/web/client/vendor/xterm.js",
        "plugins/web/client/vendor/xterm.css",
        "plugins/web/client/vendor/addon-fit.js",
        "plugins/web/client/vendor/manifest.json",
        "plugins/web/client/vendor/LICENSE.xterm",
        "plugins/web/client/vendor/LICENSE.addon-fit",
        "/workspace/plugins/web/client/vendor/xterm.js",
    ] {
        assert!(!is_retired_frontend_file(Path::new(path)), "{path}");
    }
    for path in [
        "plugins/web/client/vendor/other.js",
        "plugins/web/client/vendor/nested/xterm.js",
        "plugins/web/sdk/vendor/xterm.js",
    ] {
        assert!(is_retired_frontend_file(Path::new(path)), "{path}");
    }
    for path in [
        "plugins/web/client/vendor/xterm.js.map",
        "plugins/web/client/vendor/nested/xterm.js",
    ] {
        let file = yolo_architecture_foundation::SourceFile {
            absolute: path.into(),
            relative: path.into(),
            text: String::new(),
        };
        let mut violations = Vec::new();
        inspect(&file, &mut violations);
        assert!(
            violations
                .iter()
                .any(|violation| violation.message.contains("unapproved asset")),
            "{path}"
        );
    }
    assert!(!is_handwritten(Path::new(
        "plugins/web/client/vendor/xterm.css"
    )));
    assert!(!is_handwritten(Path::new(
        "plugins/web/client/vendor/manifest.json"
    )));
}
