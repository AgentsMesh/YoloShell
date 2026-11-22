use crate::commands::print_json;
use crate::install_git;
use crate::install_registry::{
    DEFAULT_REGISTRY, RegistrySpec, is_plugin_id, load_spec, specs_directory,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, RegistryRecord};

#[derive(Serialize)]
struct Row<'a> {
    id: &'a str,
    name: Option<&'a str>,
    registry_version: Option<&'a str>,
    installed: Option<&'a RegistryRecord>,
}

pub(super) fn list(
    paths: &RegistryPaths,
    installed: bool,
    repository: Option<&str>,
    json: bool,
) -> Result<()> {
    let local = Registry::load(paths)?;
    // Existing JSON consumers use these records as an offline inventory of trusted plugins.
    if installed || (json && repository.is_none()) {
        return print_installed(&local, json);
    }
    let repository = repository.unwrap_or(DEFAULT_REGISTRY);
    let mut specs = Vec::new();
    let discovery = collect_specs(repository, &mut specs).context(
        "could not read the complete plugin catalogue. Use `yolo plugin list --installed` for the local inventory",
    );
    if discovery.is_err() && (json || specs.is_empty()) {
        if !json {
            print_installed(&local, false)?;
        }
        return discovery;
    }
    let mut rows = BTreeMap::new();
    for record in local.records() {
        rows.insert(
            record.id.as_str(),
            Row {
                id: &record.id,
                name: None,
                registry_version: None,
                installed: Some(record),
            },
        );
    }
    for spec in &specs {
        rows.insert(
            spec.id.as_str(),
            Row {
                id: &spec.id,
                name: Some(&spec.name),
                registry_version: Some(&spec.version),
                installed: local.get(&spec.id),
            },
        );
    }
    let rows: Vec<_> = rows.into_values().collect();
    if json {
        print_json(&rows)?;
    } else {
        print_catalogue(&rows, repository);
    }
    discovery
}

fn collect_specs(repository: &str, specs: &mut Vec<RegistrySpec>) -> Result<()> {
    let temporary = tempfile::tempdir().context("create temporary plugin catalogue directory")?;
    let checkout = temporary.path().join("registry");
    install_git::clone_registry(repository, &checkout)?;
    let mut failures = Vec::new();
    for entry in specs_directory(&checkout)?.read_dir()? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        if let Some(id) = name.to_str().filter(|id| is_plugin_id(id)) {
            match load_spec(&checkout, id, None) {
                Ok(spec) => specs.push(spec),
                Err(error) => failures.push(format!("{id}: {error:#}")),
            }
        }
    }
    anyhow::ensure!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

fn print_installed(local: &Registry, json: bool) -> Result<()> {
    if json {
        print_json(&local.records().collect::<Vec<_>>())?;
    } else if local.records().next().is_none() {
        println!("No plugins installed.");
    } else {
        for record in local.records() {
            println!(
                "{} {} {}",
                record.id,
                local_state(record),
                record.source.display()
            );
        }
    }
    Ok(())
}

fn local_state(record: &RegistryRecord) -> &'static str {
    if record.enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn print_catalogue(rows: &[Row<'_>], repository: &str) {
    if rows.is_empty() {
        println!("No plugins installed or published in this registry.");
        return;
    }
    let id_width = rows
        .iter()
        .map(|row| row.id.len())
        .max()
        .unwrap_or(0)
        .max("PLUGIN".len());
    let version_width = rows
        .iter()
        .filter_map(|row| row.registry_version)
        .map(str::len)
        .max()
        .unwrap_or(0)
        .max("REGISTRY VERSION".len());
    println!(
        "{:<id_width$}  {:<version_width$}  {:<13}  NAME",
        "PLUGIN", "REGISTRY VERSION", "LOCAL STATE"
    );
    for row in rows {
        let state = row.installed.map_or("not installed", local_state);
        let name = row.name.unwrap_or("-").replace(char::is_control, " ");
        println!(
            "{:<id_width$}  {:<version_width$}  {state:<13}  {name}",
            row.id,
            row.registry_version.unwrap_or("-")
        );
    }
    if let Some(first) = rows.iter().find(|row| row.installed.is_none()) {
        if repository == DEFAULT_REGISTRY {
            println!(
                "\nReview {} with `yolo plugin install {}`.",
                first.id, first.id
            );
        } else {
            println!(
                "\nPass the same --registry URL to `yolo plugin install <id>` to review a plugin."
            );
        }
    }
}
