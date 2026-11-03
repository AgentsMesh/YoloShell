use crate::*;

#[test]
fn persisted_catalog_rejects_duplicate_missing_nil_and_unbounded_identities() {
    let original = TabCatalogDocument::from(TabCatalog::default());
    let mut empty = original.clone();
    empty.entries.clear();
    assert!(TabCatalog::try_from(empty).is_err());
    let mut oversized = original.clone();
    oversized.entries = vec![original.entries[0].clone(); MAX_TABS + 1];
    assert!(TabCatalog::try_from(oversized).is_err());
    let mut duplicate = original.clone();
    duplicate.entries.push(duplicate.entries[0].clone());
    assert!(TabCatalog::try_from(duplicate).is_err());
    let mut nil = original.clone();
    nil.entries[0].id = uuid::Uuid::nil();
    assert!(TabCatalog::try_from(nil).is_err());
    let mut missing = original.clone();
    missing.active = uuid::Uuid::new_v4();
    assert!(TabCatalog::try_from(missing).is_err());
    let mut duplicate_name = original.clone();
    let mut second = duplicate_name.entries[0].clone();
    second.id = uuid::Uuid::new_v4();
    duplicate_name.entries.push(second);
    assert!(TabCatalog::try_from(duplicate_name).is_ok());
    let mut bad_name = original;
    bad_name.entries[0].name.clear();
    assert!(TabCatalog::try_from(bad_name).is_err());
}
