use crate::constants::{FRONTEND_TERMS, MatchStyle, joined};
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn accepts_native_layers_and_centralized_values() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/constants.rs",
        "pub const STATE: &str = \"ready_state\";\n",
    );
    fixture.write(
        "build_system/coverage/collector/src/constants.rs",
        "pub const FLAG: &str = \"--coverage-mode\";\n",
    );
    fixture.write("core/src/lib.rs", "pub fn healthy() -> bool { true }\n");
    assert_eq!(messages(&fixture), Vec::<String>::new());
}

#[test]
fn rejects_retired_identity_and_old_frontend_stack() {
    let fixture = Fixture::new();
    let retired = joined(&["con", "flux"]);
    fixture.write("docs/brand.md", format!("old={retired}\n"));
    fixture.write(&format!("docs/{retired}-history.md"), "retired\n");
    for (index, rule) in FRONTEND_TERMS.iter().enumerate() {
        let term = joined(rule.parts);
        let reference = match rule.style {
            MatchStyle::Package => format!("@{term}"),
            MatchStyle::Runtime => format!("{term}.js"),
            _ => term,
        };
        fixture.write(
            &format!("docs/dependency-{index}.md"),
            format!("{reference}\n"),
        );
    }
    let result = messages(&fixture);
    let retired_count = result
        .iter()
        .filter(|entry| entry.contains("retired product identity"))
        .count();
    assert_eq!(retired_count, 2);
    assert_eq!(
        result
            .iter()
            .filter(|entry| entry.contains("frontend dependency"))
            .count(),
        FRONTEND_TERMS.len()
    );
}

#[test]
fn permits_agent_package_evidence_but_rejects_launching_old_runtime() {
    let fixture = Fixture::new();
    let runtime = joined(&["no", "de"]);
    fixture.write(
        "core/src/agent/evidence.rs",
        format!("const PATH: &str = \"/opt/{runtime}_modules/agent\";\n"),
    );
    fixture.write(
        "core/src/launcher.rs",
        format!("fn launch() {{ Command::new(\"{runtime}\"); }}\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("frontend dependency"));
}

#[test]
fn protects_rust_runtime_from_ui_dependencies() {
    let fixture = Fixture::new();
    let rust_ui = joined(&["app", "kit"]);
    fixture.write("core/src/view.rs", format!("use {rust_ui}::Window;\n"));
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(
        result
            .iter()
            .any(|entry| entry.contains("Rust runtime depends on UI"))
    );
}

#[test]
fn rejects_cli_and_known_protocol_literals_outside_constant_files() {
    let fixture = Fixture::new();
    let method = joined(&["acquire", "_writer"]);
    fixture.write(
        "core/src/protocol.rs",
        format!("pub const METHOD: &str = \"{method}\";\n"),
    );
    fixture.write(
        "core/src/service.rs",
        format!(
            "let method = \"{method}\";\nlet flag = \"{}home\";\n",
            joined(&["-", "-"])
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("centralized constant"))
    );
}
