use crate::install::Plan;
use crate::render;
use crate::status::Row;
use yolo_hook_installation::{
    HookEventSpec, HookInstallSpec, Installation, IntegrationStatus, Preview,
};

static SPEC: HookInstallSpec = HookInstallSpec {
    provider: "demo",
    display_name: "Demo Agent",
    file_name: "settings.json",
    subdirectory: "",
    environment_name: "DEMO_HOME",
    directory_name: ".demo",
    hook_arguments: "provider-hook --provider demo",
    managed_hook_commands: &["provider-hook --provider demo"],
    events: &[HookEventSpec::new("SessionStart")],
    restricted_keys: &[],
    disabled_paths: &[],
    requires_provider_trust: true,
};

fn preview(status: IntegrationStatus) -> Preview {
    Preview {
        provider: SPEC.provider.into(),
        path: "/tmp/demo/settings.json".into(),
        revision: "revision-a".into(),
        status,
        requires_provider_trust: SPEC.requires_provider_trust,
    }
}

fn plan(status: IntegrationStatus) -> Plan {
    Plan {
        spec: &SPEC,
        home: "/tmp/demo".into(),
        preview: preview(status),
    }
}

#[test]
fn status_labels_are_human_readable() {
    assert_eq!(render::label(IntegrationStatus::Missing), "not installed");
    assert_eq!(render::label(IntegrationStatus::Configured), "installed");
    assert_eq!(render::label(IntegrationStatus::Restricted), "disabled");
}

#[test]
fn status_line_reports_trust_and_path() {
    let row = Row {
        spec: &SPEC,
        preview: preview(IntegrationStatus::Missing),
    };
    assert_eq!(
        render::status_line(&row),
        "demo\tDemo Agent\tnot installed\trequired\t/tmp/demo/settings.json"
    );
}

#[test]
fn preview_explains_managed_scope_and_trust() {
    let lines = render::preview_lines(&plan(IntegrationStatus::Missing));
    assert!(lines.contains(&"Demo Agent".to_string()));
    assert!(lines.contains(&"  file:    /tmp/demo/settings.json".to_string()));
    assert!(lines.contains(&"  status:  not installed".to_string()));
    assert!(lines.contains(&"  events:  1 managed callbacks".to_string()));
    assert!(lines.iter().any(|line| line.starts_with("  trust:")));
}

#[test]
fn installed_lines_include_the_backup() {
    let installation = Installation {
        preview: preview(IntegrationStatus::Configured),
        changed: true,
        backup: Some("/tmp/demo/settings.json.backup".into()),
    };
    let lines = render::installed_lines(&SPEC, &installation);
    assert_eq!(
        lines[0],
        "Demo Agent: installed at /tmp/demo/settings.json"
    );
    assert_eq!(lines[1], "  backup: /tmp/demo/settings.json.backup");
}

#[test]
fn json_documents_expose_the_machine_status() {
    let row = Row {
        spec: &SPEC,
        preview: preview(IntegrationStatus::Missing),
    };
    let document = render::status_json(&row);
    assert_eq!(document["provider"], "demo");
    assert_eq!(document["status"], "missing");
    assert_eq!(document["requires_provider_trust"], true);
    assert_eq!(document["path"], "/tmp/demo/settings.json");
}
