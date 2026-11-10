use super::*;

const AGGREGATE: &str = "Aggregate";
const ALTERNATE: &str = "Alternate";
const CANONICAL: &str = "RetainedExactProcess";
const CRATE: &str = "crate";
const MIXED: &str = "Mixed";

#[test]
fn mixed_alias_targets_are_indeterminate() {
    let mut catalog = catalog();
    alias(&mut catalog, AGGREGATE, false);
    alias(&mut catalog, CANONICAL, true);
    assert!(catalog.contains(&qualified(MIXED), &path(CRATE)) == Match::Uncertain);
}

#[test]
fn equivalent_aggregate_alias_targets_keep_their_classification() {
    let mut catalog = catalog();
    alias(&mut catalog, AGGREGATE, false);
    alias(&mut catalog, ALTERNATE, false);
    assert!(catalog.contains(&qualified(MIXED), &path(CRATE)) == Match::Aggregate);
}

#[test]
fn exact_aggregate_and_canonical_alias_are_indeterminate() {
    let mut catalog = catalog();
    catalog.insert(path(CRATE), MIXED, &path(CRATE), false);
    alias(&mut catalog, CANONICAL, true);
    assert!(catalog.contains(&qualified(MIXED), &path(CRATE)) == Match::Uncertain);
}

fn catalog() -> Catalog {
    let mut catalog = Catalog::default();
    catalog.insert(path(CRATE), AGGREGATE, &path(CRATE), false);
    catalog.insert(path(CRATE), ALTERNATE, &path(CRATE), false);
    catalog.insert(path(CRATE), CANONICAL, &path(CRATE), true);
    catalog
}

fn alias(catalog: &mut Catalog, target: &str, canonical: bool) {
    catalog.alias_type(
        &qualified(MIXED),
        &qualified(target),
        &path(CRATE),
        canonical,
    );
}

fn path(value: &str) -> Vec<String> {
    vec![value.to_owned()]
}

fn qualified(value: &str) -> Vec<String> {
    vec![CRATE.to_owned(), value.to_owned()]
}
