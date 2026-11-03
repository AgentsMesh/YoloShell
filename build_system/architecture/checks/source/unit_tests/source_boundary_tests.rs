use super::*;

#[test]
fn source_classification_distinguishes_root_component_and_external_sources() {
    for (path, rust, product) in [
        ("core/src/lib.rs", true, true),
        ("core/crates/example/src/lib.rs", true, true),
        ("core/crates/example/unit_tests/check.rs", true, false),
        (
            "core/crates/example/integration_tests/check.rs",
            true,
            false,
        ),
        ("core/crates/example/test_support/data.rs", true, false),
        ("core/crates/example/src/lib.swift", false, false),
        ("core/other.rs", false, false),
        ("build_system/example/src/lib.rs", false, false),
        ("outside.rs", false, false),
    ] {
        assert_eq!(is_rust_source(path), rust, "{path}");
        assert_eq!(is_rust_product_source(path), product, "{path}");
    }
}

#[test]
fn implementation_classification_excludes_each_nonproduction_subtree() {
    for prefix in ["core/crates/example", "build_system/example"] {
        for directory in [UNIT_TEST_DIRECTORY_NAME, INTEGRATION_TEST_DIRECTORY_NAME] {
            assert!(!rust_implementation_source(&format!(
                "{prefix}/{directory}/src/helper.rs"
            )));
        }
    }
    for path in [
        "build_system/example/test_support/src/helper.rs",
        "core/crates/example/other/helper.rs",
        "build_system/example/other/helper.rs",
        "outside/src/helper.rs",
        "core/src/readme.md",
    ] {
        assert!(!rust_implementation_source(path), "{path}");
    }
}

#[test]
fn support_placement_handles_extensions_stems_and_both_test_directories() {
    for path in [
        "build_system/example/unit_tests/check.rs",
        "build_system/example/integration_tests/check.rs",
        "build_system/example/unit_tests/fixture.txt",
        "build_system/example/integration_tests/fixture_test.rs",
        "core/crates/example/unit_tests/fixture.rs",
        "build_system/example/src/fixture.rs",
        "build_system/example/unit_tests/",
    ] {
        assert!(!misplaced_test_support(path), "{path}");
    }
    assert!(misplaced_test_support(
        "build_system/example/integration_tests/data_fixture.rs"
    ));
}

#[test]
fn handwritten_and_retired_name_checks_handle_extensionless_and_root_paths() {
    for path in [
        "/",
        "",
        "unknown",
        "picture.png",
        "Cargo.lock",
        "MODULE.bazel.lock",
    ] {
        assert!(!is_handwritten(Path::new(path)), "{path}");
    }
    assert!(is_handwritten(Path::new("BUILD")));
    assert!(is_handwritten(Path::new("src/check.rs")));
    assert!(!is_rust_test_source("helper.rs"));
    assert!(!is_rust_test_source("/"));
    assert_eq!(file_name(Path::new("/")), "");
    assert_eq!(file_stem(Path::new("/")), "");
    for parts in RETIRED_FRONTEND_FILE_PARTS {
        assert!(is_retired_frontend_file(Path::new(&joined(parts))));
    }
    for parts in RETIRED_FRONTEND_DIRECTORY_PARTS {
        assert!(is_retired_frontend_directory(Path::new(&joined(parts))));
    }
    for path in ["/", "ordinary", "ordinary.rs"] {
        assert!(!is_retired_frontend_file(Path::new(path)));
        assert!(!is_retired_frontend_directory(Path::new(path)));
    }
}

#[test]
fn web_plugin_paths_allow_only_typescript_and_bun_metadata() {
    for path in [
        "plugins/web/sdk/src/client.ts",
        "plugins/web/client/scripts/plugin.tsx",
        "plugins/web/client/package.json",
        "plugins/web/client/bun.lock",
        "plugins/web/client/tsconfig.json",
        "/workspace/plugins/web/client/server/index.ts",
    ] {
        assert!(!is_retired_frontend_file(Path::new(path)), "{path}");
    }
    for path in [
        "plugins/web/gateway/src/server.ts",
        "plugins/web/client/scripts/plugin.js",
        "plugins/web/client/package-lock.json",
        "core/crates/example/src/client.ts",
        "plugins/web/client-extra/index.ts",
    ] {
        assert!(is_retired_frontend_file(Path::new(path)), "{path}");
    }
    assert!(is_handwritten(Path::new(
        "plugins/web/client/scripts/plugin.ts"
    )));
    assert!(is_handwritten(Path::new(
        "plugins/web/client/scripts/plugin.tsx"
    )));
}

#[test]
fn retired_frontend_dependencies_are_detected_in_paths_and_runtime_metadata() {
    for parts in [["re", "act"], ["x", "term"], ["no", "de"]] {
        let term = parts.concat();
        let relative = format!("build_system/{term}/source.rs");
        let file = yolo_architecture_foundation::SourceFile {
            absolute: relative.clone().into(),
            relative,
            text: String::new(),
        };
        let mut violations = Vec::new();
        crate::brand_rules::inspect(&file, &mut violations);
        assert!(violations.iter().any(|item| item.message.contains(&term)));
    }
    let file = yolo_architecture_foundation::SourceFile {
        absolute: "build_system/BUILD.bazel".into(),
        relative: "build_system/BUILD.bazel".into(),
        text: ["load(\"@rules_", "no", "de//:defs.bzl\", \"tool\")"].concat(),
    };
    let mut violations = Vec::new();
    crate::brand_rules::inspect(&file, &mut violations);
    assert!(
        violations
            .iter()
            .any(|item| item.message.contains(&["no", "de"].concat()))
    );
}

#[test]
fn runtime_paths_distinguish_build_tools_from_native_rust_components() {
    let term = ["no", "de"].concat();
    for (relative, expected) in [
        (format!("build_system/{term}/src/main.rs"), true),
        (format!("build_system/{term}-tool/source.bzl"), true),
        (format!("scripts/{term}.sh"), true),
        (format!("build_system/{term}/SRC/MAIN.RS"), true),
        (format!("core/crates/{term}/src/lib.rs"), false),
        (format!("core/crates/{term}_tool/src/lib.rs"), false),
    ] {
        let file = yolo_architecture_foundation::SourceFile {
            absolute: relative.clone().into(),
            relative,
            text: String::new(),
        };
        let mut violations = Vec::new();
        crate::brand_rules::inspect(&file, &mut violations);
        assert_eq!(
            violations.iter().any(|item| item.message.contains(&term)),
            expected
        );
    }
}

#[test]
fn rust_launches_of_the_retired_runtime_are_rejected_outside_build_tools() {
    let term = ["no", "de"].concat();
    for text in [
        format!("std::process::Command::new(\"{term}\")"),
        format!("Command::new(\"{term}\")"),
        format!("exec {term}"),
        format!("/usr/bin/env {term}"),
    ] {
        let file = yolo_architecture_foundation::SourceFile {
            absolute: "core/crates/example/src/launcher.rs".into(),
            relative: "core/crates/example/src/launcher.rs".into(),
            text: text.clone(),
        };
        let mut violations = Vec::new();
        crate::brand_rules::inspect(&file, &mut violations);
        assert!(
            violations.iter().any(|item| item.message.contains(&term)),
            "{text}"
        );
    }
}
