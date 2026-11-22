use super::{BINARY_ENV, INSTALL_COMMAND, JSON_ARGUMENT, LIST_COMMAND};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Output};

#[path = "../test_support/plugin_discovery_constants.rs"]
mod constants;
#[path = "../test_support/plugin_discovery.rs"]
mod fixture;
use constants::*;
use fixture::{Fixture, success};

fn rows(output: Output) -> Vec<Value> {
    serde_json::from_str(success(&output)).unwrap()
}

#[test]
fn install_links_immediately_and_dry_run_only_reports_the_verified_source() {
    let fixture = Fixture::new();
    fixture.publish_source();
    let details = fixture.plugin(&[INSTALL_COMMAND, WEB, DRY_RUN, JSON_ARGUMENT]);
    let preview: Value = serde_json::from_str(success(&details)).unwrap();
    assert_eq!(preview["version"], LATEST);
    assert!(!fixture.account().exists());

    let install = fixture.plugin(&[INSTALL_COMMAND, WEB]);
    assert!(success(&install).starts_with(&format!("installed {WEB} {LATEST} at ")));
    let local = rows(fixture.plugin(&[LIST_COMMAND, INSTALLED, JSON_ARGUMENT]));
    let [record] = local.as_slice() else {
        panic!("expected one installed plugin")
    };
    assert_eq!(record["id"], WEB);
    assert_eq!(record["enabled"], true);
    assert_eq!(record["digest"], preview["digest"]);

    success(&fixture.plugin(&[DISABLE, WEB]));
    let registry = fixture.account().join("config/plugins.json");
    let before = fs::read(&registry).unwrap();
    let dry_run = fixture.plugin(&[INSTALL_COMMAND, WEB, TRUST, DRY_RUN, JSON_ARGUMENT]);
    assert_eq!(
        serde_json::from_str::<Value>(success(&dry_run)).unwrap(),
        preview
    );
    assert_eq!(fs::read(&registry).unwrap(), before);
}

#[test]
fn legacy_install_scripts_receive_the_verified_details_and_an_enabled_plugin() {
    let fixture = Fixture::new();
    fixture.publish_source();
    let install = fixture.plugin(&[INSTALL_COMMAND, WEB, TRUST, JSON_ARGUMENT]);
    let details: Value = serde_json::from_str(success(&install)).unwrap();
    let records = rows(fixture.plugin(&[LIST_COMMAND, INSTALLED, JSON_ARGUMENT]));
    let [record] = records.as_slice() else {
        panic!("expected one installed plugin")
    };
    assert_eq!(record["enabled"], true);
    assert_eq!(details["manifest"]["id"], WEB);
    assert_eq!(details["digest"], record["digest"]);
}

#[test]
fn first_visit_discovers_the_latest_registry_version_without_installing() {
    let fixture = Fixture::new();
    for args in [&[][..], &[LIST_COMMAND][..]] {
        let output = fixture.plugin(args);
        let text = success(&output);
        assert!(
            text.contains(WEB) && text.contains(LATEST) && text.contains("not installed"),
            "{text}"
        );
    }
    let catalog = rows(fixture.plugin(&[LIST_COMMAND, REGISTRY, JSON_ARGUMENT]));
    assert_eq!(
        catalog,
        [json!({"id": WEB, "name": WEB, "registry_version": LATEST, "installed": null})]
    );
    fixture.disconnect_registry();
    fs::remove_file(&fixture.trace).unwrap();
    for args in [
        &[LIST_COMMAND, JSON_ARGUMENT][..],
        &[LIST_COMMAND, INSTALLED, JSON_ARGUMENT][..],
    ] {
        assert!(rows(fixture.plugin(args)).is_empty());
    }
    assert!(
        !fixture.trace.exists(),
        "installed-only queries invoked git"
    );
}

#[test]
fn linked_plugins_survive_a_catalogue_outage_and_share_a_row_with_their_registry_entry() {
    let fixture = Fixture::new();
    fixture.link(WEB);
    fixture.link(LOCAL);
    success(&fixture.plugin(&[DISABLE, WEB]));
    let local = rows(fixture.plugin(&[LIST_COMMAND, JSON_ARGUMENT]));
    let catalog = rows(fixture.plugin(&[
        LIST_COMMAND,
        REGISTRY,
        fixture.registry.to_str().unwrap(),
        JSON_ARGUMENT,
    ]));
    assert_eq!(
        catalog
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [LOCAL, WEB]
    );
    for (row, record) in catalog.iter().zip(&local) {
        assert_eq!(&row["installed"], record);
        assert!(Path::new(record["source"].as_str().unwrap()).is_absolute());
        assert!(record["digest"].is_string());
    }
    assert!(catalog[0]["registry_version"].is_null());
    assert_eq!(catalog[1]["registry_version"], LATEST);
    assert_eq!(catalog[1]["installed"]["enabled"], false);
    fixture.disconnect_registry();
    let missing = fixture.registry.to_str().unwrap();
    let unavailable = fixture.plugin(&[LIST_COMMAND, REGISTRY, missing]);
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stdout).contains(LOCAL));
    assert!(
        String::from_utf8_lossy(&unavailable.stderr)
            .to_lowercase()
            .contains("catalogue")
    );
    let unavailable_json = fixture.plugin(&[LIST_COMMAND, REGISTRY, missing, JSON_ARGUMENT]);
    assert!(!unavailable_json.status.success());
    assert!(unavailable_json.stdout.is_empty());
}

#[test]
fn a_draft_spec_keeps_valid_plugins_visible_and_marks_machine_output_incomplete() {
    let fixture = Fixture::new();
    fixture.add_draft(DRAFT);
    let registry = fixture.registry.to_str().unwrap();
    let text = fixture.plugin(&[LIST_COMMAND, REGISTRY, registry]);
    assert!(!text.status.success());
    let rows = String::from_utf8(text.stdout).unwrap();
    assert!(
        rows.lines().any(|line| line.contains(WEB)
            && line.contains(LATEST)
            && line.contains("not installed")),
        "{rows}"
    );
    assert!(String::from_utf8_lossy(&text.stderr).contains(DRAFT));

    let json = fixture.plugin(&[LIST_COMMAND, REGISTRY, registry, JSON_ARGUMENT]);
    assert!(!json.status.success());
    assert!(json.stdout.is_empty());
    assert!(String::from_utf8_lossy(&json.stderr).contains(DRAFT));
}
