use std::collections::BTreeSet;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::starlark_baseline::tuple_entries;

use crate::constants::TEST_DIRECTORIES;
use crate::module_closure::{IncludeSite, include_sites, resolve};

const BASELINE_FILES: &[&str] = &[
    "build_system/architecture/include_composition_baseline_runtime.bzl",
    "build_system/architecture/include_composition_baseline_platform.bzl",
];
const BASELINE_ASSIGNMENT: &str = "INCLUDE_COMPOSITION_BASELINE";
const GENERATED_COMPONENT: &str = "generated";
const SOURCE_COMPONENT: &str = "src";
const COMPOSITION_POLICY: &str =
    "include! composition is limited to generated contracts and recorded sites";
const DYNAMIC_POLICY: &str =
    "include! arguments must be a literal path so composition stays reviewable";
const STALE_POLICY: &str = "stale include composition baseline entry";
const MALFORMED_POLICY: &str =
    "include composition baseline entries need a source and target string";

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let baseline = baseline_sites(files, violations);
    let mut present = BTreeSet::new();
    for file in files.iter().filter(|file| file.relative.ends_with(".rs")) {
        for site in include_sites(&file.text) {
            let IncludeSite::Literal(target) = site else {
                violations.push(Violation::file(&file.relative, DYNAMIC_POLICY));
                continue;
            };
            let target = resolve(&file.relative, &target);
            if allowed(&file.relative, &target) {
                continue;
            }
            let site = (file.relative.clone(), target);
            if baseline.contains(&site) {
                present.insert(site);
                continue;
            }
            violations.push(Violation::file(
                &file.relative,
                format!("{COMPOSITION_POLICY}: {}", site.1),
            ));
        }
    }
    for (source, target) in baseline.difference(&present) {
        violations.push(Violation::file(source, format!("{STALE_POLICY}: {target}")));
    }
}

fn allowed(source: &str, target: &str) -> bool {
    if target
        .split('/')
        .any(|component| component == GENERATED_COMPONENT)
    {
        return true;
    }
    let Some(test_root) = root_before(target, |component| TEST_DIRECTORIES.contains(&component))
    else {
        return false;
    };
    source_root(source).as_deref() == Some(test_root.as_str())
}

fn source_root(path: &str) -> Option<String> {
    root_before(path, |component| {
        component == SOURCE_COMPONENT || TEST_DIRECTORIES.contains(&component)
    })
}

fn root_before(path: &str, boundary: impl Fn(&str) -> bool) -> Option<String> {
    let components = path.split('/').collect::<Vec<_>>();
    let index = components.iter().position(|component| boundary(component))?;
    Some(components[..index].join("/"))
}

fn baseline_sites(
    files: &[SourceFile],
    violations: &mut Vec<Violation>,
) -> BTreeSet<(String, String)> {
    let mut sites = BTreeSet::new();
    for path in BASELINE_FILES {
        let Some(baseline) = files.iter().find(|file| file.relative == *path) else {
            continue;
        };
        let entries = match tuple_entries(&baseline.text, BASELINE_ASSIGNMENT, 2) {
            Ok(entries) => entries,
            Err(error) => {
                violations.push(Violation::file(
                    &baseline.relative,
                    format!("{MALFORMED_POLICY}: line {}", error.line),
                ));
                continue;
            }
        };
        for entry in entries {
            let (source, target) = (&entry.values[0], &entry.values[1]);
            sites.insert((source.clone(), resolve(source, target)));
        }
    }
    sites
}
