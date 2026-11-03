use yolo_architecture_foundation::{SourceFile, model::Violation};

fn inspect(relative: &str, text: String) -> Vec<Violation> {
    let file = SourceFile {
        absolute: relative.into(),
        relative: relative.into(),
        text,
    };
    let mut violations = Vec::new();
    crate::brand_rules::inspect(&file, &mut violations);
    violations
}

#[test]
fn vendored_emulator_is_scoped_to_the_exact_allowlist() {
    let term = ["x", "term"].concat();
    let allowed = inspect(
        "plugins/web/client/vendor/xterm.js",
        format!("require(\"@{term}/{term}\");"),
    );
    assert!(
        !allowed
            .iter()
            .any(|violation| violation.message.contains(&term)),
        "{allowed:?}"
    );
    let rejected = inspect(
        "plugins/web/client/scripts/plugin.ts",
        format!("const packageName = \"@{term}/{term}\";"),
    );
    assert!(
        rejected
            .iter()
            .any(|violation| violation.message.contains(&term)),
        "{rejected:?}"
    );
}

#[test]
fn web_plugin_typescript_may_use_builtin_modules() {
    let term = ["no", "de"].concat();
    let allowed = inspect(
        "plugins/web/client/scripts/plugin.ts",
        format!("import {{ readFileSync }} from \"{term}:fs\";"),
    );
    assert!(
        !allowed
            .iter()
            .any(|violation| violation.message.contains(&term))
    );
    for (relative, text) in [
        (
            "core/crates/example/src/launcher.ts",
            format!("import \"{term}:fs\";"),
        ),
        (
            "plugins/web/client/scripts/plugin.ts",
            format!("const path = \"{term}_modules/x\";"),
        ),
    ] {
        let violations = inspect(relative, text);
        assert!(
            violations.iter().any(|item| item.message.contains(&term)),
            "{relative}"
        );
    }
}
