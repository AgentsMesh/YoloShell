use super::*;
use crate::model::ScopedType;

const OWNER: &str = "owner";
const CHILD: &str = "child";
const FOREIGN: &str = "foreign";
const FIRST: &str = "First";
const SECOND: &str = "Second";

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn incomparable_or_inaccessible_boundaries_cannot_widen_visibility() {
    let owner = path(&[OWNER]);
    let child = path(&[OWNER, CHILD]);
    let foreign = path(&[FOREIGN]);
    assert_eq!(narrowest(Some(&owner), &child, &child), Some(child.clone()));
    assert_eq!(narrowest(Some(&child), &owner, &child), Some(child.clone()));
    assert_eq!(narrowest(None, &foreign, &child), None);
    assert_eq!(narrowest(Some(&foreign), &owner, &child), None);
}

#[test]
fn incomplete_or_inaccessible_aliases_cannot_supply_a_boundary() {
    let scope = path(&[OWNER]);
    let mut alias = ModuleAlias {
        destination: Vec::new(),
        target: path(&[OWNER, FIRST]),
        visible_from: scope.clone(),
        module: false,
        canonical: false,
    };
    assert_eq!(alias_boundary(&alias, None, &scope), None);
    alias.destination = path(&[OWNER, SECOND]);
    assert_eq!(alias_boundary(&alias, None, &scope), Some(None));
    alias.visible_from = path(&[FOREIGN]);
    assert_eq!(alias_boundary(&alias, None, &scope), None);
}

#[test]
fn cycles_never_manufacture_an_aggregate_declaration() {
    let mut catalog = Catalog::default();
    let scope = path(&[OWNER]);
    let first = path(&[OWNER, FIRST]);
    let second = path(&[OWNER, SECOND]);
    catalog.alias_type(&first, &second, &scope, false);
    catalog.alias_type(&second, &first, &scope, false);
    assert_eq!(catalog.aggregate_boundaries(&first, &scope), None);
}

#[test]
fn canonical_hidden_and_non_declaration_entries_are_not_aggregate_authority() {
    let scope = path(&[OWNER]);
    let target = path(&[OWNER, FIRST]);
    for entry in [
        ScopedType {
            visible_from: scope.clone(),
            canonical: true,
            declaration: true,
        },
        ScopedType {
            visible_from: scope.clone(),
            canonical: false,
            declaration: false,
        },
        ScopedType {
            visible_from: path(&[FOREIGN]),
            canonical: false,
            declaration: true,
        },
    ] {
        let mut catalog = Catalog::default();
        catalog
            .exact_scopes
            .insert(target.clone(), BTreeSet::from([entry]));
        assert_eq!(catalog.aggregate_boundaries(&target, &scope), None);
    }
}

#[test]
fn mismatched_alias_index_cannot_redirect_authority() {
    let scope = path(&[OWNER]);
    let target = path(&[OWNER, FIRST]);
    let mut catalog = Catalog::default();
    catalog.aliases_by_destination.insert(
        target.clone(),
        BTreeSet::from([ModuleAlias {
            destination: path(&[FOREIGN, SECOND]),
            target: target.clone(),
            visible_from: scope.clone(),
            module: false,
            canonical: false,
        }]),
    );
    assert_eq!(catalog.aggregate_boundaries(&target, &scope), None);
}
