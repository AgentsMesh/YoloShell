use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::starlark_lists::{Element, list_entries, string_assignments};

pub(crate) const ROLES_FILE: &str = "build_system/architecture/hub_roles.bzl";
pub(crate) const TOPOLOGY_FILE: &str = "build_system/architecture/topology.bzl";
pub(crate) const HUB_LABEL_PREFIX: &str = "//core/crates/hub/";
const PUBLIC_LIST: &str = "PUBLIC_COMPONENTS";
const CLIENT_LIST: &str = "CLIENT_COMPONENTS";
const ENTRY_LIST: &str = "HUB_ENTRY_COMPONENTS";
const RUNTIME_LIST: &str = "RUNTIME_COMPONENTS";
const DUPLICATE_POLICY: &str = "Hub target is classified more than once";
const CONFLICT_POLICY: &str = "Hub package mixes public, client, and runtime roles";
const MALFORMED_POLICY: &str = "Hub role lists need string or topology labels";
const UNRESOLVED_POLICY: &str = "Hub role label is not defined in the topology manifest";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Role {
    Client,
    Entry,
    Public,
    Runtime,
}

#[derive(Default)]
pub(crate) struct Classification {
    labels: BTreeMap<String, BTreeSet<Role>>,
}

impl Classification {
    pub(crate) fn labels(&self) -> impl Iterator<Item = &String> {
        self.labels.keys()
    }

    pub(crate) fn classified(&self, label: &str) -> bool {
        self.labels.contains_key(label)
    }

    pub(crate) fn packages(&self) -> BTreeSet<&str> {
        self.labels
            .keys()
            .filter_map(|label| label.split_once(':').map(|(package, _)| package))
            .filter(|package| package.starts_with(HUB_LABEL_PREFIX))
            .collect()
    }
}

pub(crate) fn read(
    files: &[SourceFile],
    violations: &mut Vec<Violation>,
) -> Option<Classification> {
    let roles_file = files.iter().find(|file| file.relative == ROLES_FILE)?;
    let topology = files
        .iter()
        .find(|file| file.relative == TOPOLOGY_FILE)
        .map(|file| string_assignments(&file.text))
        .unwrap_or_default();
    let mut labels: BTreeMap<String, BTreeSet<Role>> = BTreeMap::new();
    for (assignment, role) in [
        (PUBLIC_LIST, Role::Public),
        (CLIENT_LIST, Role::Client),
        (ENTRY_LIST, Role::Entry),
        (RUNTIME_LIST, Role::Runtime),
    ] {
        let items = match list_entries(&roles_file.text, assignment) {
            Ok(items) => items,
            Err(error) => {
                violations.push(Violation::line(
                    &roles_file.relative,
                    error.line,
                    MALFORMED_POLICY,
                ));
                continue;
            }
        };
        for item in items {
            let label = match item.element {
                Element::String(label) => label,
                Element::Name(name) => match topology.get(&name) {
                    Some(label) => label.clone(),
                    None => {
                        violations.push(Violation::line(
                            &roles_file.relative,
                            item.line,
                            format!("{UNRESOLVED_POLICY}: {name}"),
                        ));
                        continue;
                    }
                },
            };
            if !label.starts_with(HUB_LABEL_PREFIX) {
                continue;
            }
            let roles = labels.entry(label.clone()).or_default();
            if !roles.insert(role) {
                violations.push(Violation::line(
                    &roles_file.relative,
                    item.line,
                    format!("{DUPLICATE_POLICY}: {label}"),
                ));
            }
        }
    }
    for (package, primary) in primary_roles(&labels) {
        if primary.len() > 1 {
            violations.push(Violation::file(
                &roles_file.relative,
                format!("{CONFLICT_POLICY}: {package}"),
            ));
        }
    }
    Some(Classification { labels })
}

fn primary_roles(labels: &BTreeMap<String, BTreeSet<Role>>) -> BTreeMap<String, BTreeSet<Role>> {
    let mut packages: BTreeMap<String, BTreeSet<Role>> = BTreeMap::new();
    for (label, roles) in labels {
        let Some((package, _)) = label.split_once(':') else {
            continue;
        };
        let primary = packages.entry(package.to_owned()).or_default();
        primary.extend(
            roles
                .iter()
                .copied()
                .filter(|role| *role != Role::Entry),
        );
    }
    packages
}
