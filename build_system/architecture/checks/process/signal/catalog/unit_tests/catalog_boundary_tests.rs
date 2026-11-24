use super::{Catalog, MAXIMUM_PATH_RESOLUTION_STATES, MINIMUM_RESOLUTION_FACTOR};
use std::collections::BTreeSet;
use yolo_architecture_policy_contracts::process_signal_retained::BRIDGE_PATH_COMPONENT;

const AGGREGATE: &str = "Aggregate";
const AUTHORITY: &str = "authority";
const CHILD: &str = "child";
const DEEPER: &str = "deeper";
const FIRST_ALIAS_INDEX: usize = 0;
const NESTED: &str = "nested";
const OWNER: &str = "owner";
const PUBLIC_ALIAS: &str = "PublicAggregate";
const RESOLUTION_EXPECTATION: &str = "aggregate boundaries";
const ROOT: &str = "root";
const SHARED_ALIAS: &str = "SharedAggregate";
const TARGET_PREFIX: &str = "Target";

#[test]
fn catalog_cloning_preserves_authority_and_comparison_excludes_resolution_cache() {
    let mut catalog = Catalog::default();
    let scope = path(&[OWNER]);
    let target = declaration(&mut catalog, &[OWNER], AGGREGATE, &scope);
    let copy = catalog.clone();
    assert!(copy == catalog);
    let _ = copy.contains(&target, &scope);
    assert!(copy == catalog);
    assert!(Catalog::default() != catalog);
    catalog.alias_module(&path(&[OWNER, PUBLIC_ALIAS]), &path(&[OWNER]), &scope);
    assert!(copy != catalog);
    assert!(catalog.contains_direct(&[], &scope) == super::Match::No);
}

#[test]
fn cyclic_module_aliases_terminate_and_expanding_aliases_exhaust_a_bound() {
    let scope = path(&[OWNER]);
    let first = path(&[OWNER, CHILD]);
    let second = path(&[OWNER, NESTED]);
    let mut cyclic = Catalog::default();
    cyclic.alias_module(&first, &second, &scope);
    cyclic.alias_module(&second, &first, &scope);
    assert!(!cyclic.module_children(&first, &scope).uncertain);
    assert!(cyclic.type_children(&first, &scope).values.is_empty());

    let mut expanding = Catalog::default();
    let mut target = first.clone();
    target.push(CHILD.into());
    expanding.alias_module(&first, &target, &scope);
    assert!(expanding.module_children(&first, &scope).uncertain);
    assert!(expanding.type_children(&first, &scope).uncertain);
}

#[test]
fn type_alias_chain_retains_its_narrowest_boundary() {
    let mut catalog = Catalog::default();
    let child = path(&[OWNER, CHILD]);
    let nested = path(&[OWNER, CHILD, NESTED]);
    let target = declaration(&mut catalog, &[OWNER, CHILD, AUTHORITY], AGGREGATE, &child);
    let shared = path(&[OWNER, CHILD, NESTED, BRIDGE_PATH_COMPONENT, SHARED_ALIAS]);
    catalog.alias_type(&shared, &target, &nested, false);
    let public = path(&[OWNER, CHILD, NESTED, PUBLIC_ALIAS]);
    catalog.alias_type(&public, &shared, &child, false);
    let boundaries = catalog
        .aggregate_boundaries(&public, &path(&[OWNER, CHILD, NESTED, DEEPER]))
        .expect(RESOLUTION_EXPECTATION);
    assert_eq!(boundaries, BTreeSet::from([nested]));
}

#[test]
fn multiple_declarations_preserve_every_boundary() {
    let mut catalog = Catalog::default();
    let child = path(&[OWNER, CHILD]);
    let nested = path(&[OWNER, CHILD, NESTED]);
    let module = path(&[OWNER, CHILD, NESTED, AUTHORITY]);
    catalog.insert(module.clone(), AGGREGATE, &child, false);
    catalog.insert(module.clone(), AGGREGATE, &nested, false);
    let mut target = module;
    target.push(AGGREGATE.to_owned());
    let boundaries = catalog
        .aggregate_boundaries(&target, &path(&[OWNER, CHILD, NESTED, DEEPER]))
        .expect(RESOLUTION_EXPECTATION);
    assert_eq!(boundaries, BTreeSet::from([child.clone(), nested]));
    assert!(
        !boundaries
            .iter()
            .all(|boundary| child.starts_with(boundary))
    );
}

#[test]
fn boundary_resolution_budget_exhaustion_fails_closed() {
    let mut catalog = Catalog::default();
    let boundary = path(&[OWNER]);
    let target = declaration(&mut catalog, &[OWNER, AUTHORITY], AGGREGATE, &boundary);
    let scope = path(&[OWNER, AUTHORITY]);
    assert!(catalog.aggregate_boundaries(&target, &scope).is_some());
    let excess_alias_count = MAXIMUM_PATH_RESOLUTION_STATES + MINIMUM_RESOLUTION_FACTOR;
    for index in FIRST_ALIAS_INDEX..excess_alias_count {
        catalog.alias_type(
            &target,
            &[ROOT.to_owned(), format!("{TARGET_PREFIX}{index}")],
            &boundary,
            false,
        );
    }
    assert!(catalog.aggregate_boundaries(&target, &scope).is_none());
}

fn declaration(
    catalog: &mut Catalog,
    module: &[&str],
    name: &str,
    visible_from: &[String],
) -> Vec<String> {
    let mut target = path(module);
    catalog.insert(target.clone(), name, visible_from, false);
    target.push(name.to_owned());
    target
}

fn path(parts: &[&str]) -> Vec<String> {
    std::iter::once(ROOT.to_owned())
        .chain(parts.iter().map(|part| (*part).to_owned()))
        .collect()
}
