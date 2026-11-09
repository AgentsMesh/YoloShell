use crate::starlark_lists::{
    Element, list_entries, string_assignments, string_entries,
};

#[test]
fn reads_a_commented_string_list() {
    let source = "ROLES = [\n    \"//a:one\",  # trailing comment\n    # \"//a:commented\",\n    \"//a:two\",\n]\n";

    let items = string_entries(source, "ROLES").unwrap();
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].value, "//a:one");
    assert_eq!(items[1].value, "//a:two");
}

#[test]
fn rejects_a_non_string_role_item() {
    let source = "ROLES = [\n    \"//a:one\",\n    other,\n]\n";

    let error = string_entries(source, "ROLES").unwrap_err();
    assert_eq!(error.line, 3);
}

#[test]
fn reads_a_mixed_list_of_strings_and_names() {
    let source = "ITEM = \"//a:item\"\nLIST = [\n    ITEM,\n    \"//a:other\",\n]\n";

    let items = list_entries(source, "LIST").unwrap();
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].element, Element::Name("ITEM".into()));
    assert_eq!(items[1].element, Element::String("//a:other".into()));
}

#[test]
fn rejects_a_comprehension_as_a_flat_list() {
    let source = "LIST = [item for item in OTHER]\n";

    assert!(list_entries(source, "LIST").is_err());
}

#[test]
fn reads_uppercase_string_assignments_only() {
    let source = "ITEM = \"//a:item\"\nname = \"ignored\"\n# COMMENTED = \"//a:gone\"\n";

    let assignments = string_assignments(source);
    assert_eq!(assignments.get("ITEM"), Some(&"//a:item".to_owned()));
    assert!(!assignments.contains_key("name"));
    assert!(!assignments.contains_key("COMMENTED"));
}
