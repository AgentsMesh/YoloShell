use super::*;

const NAME: &str = "Alias";
const DECLARATION: usize = 3;
const NESTED_BLOCK: usize = 5;
const SHADOW_START: usize = 10;
const SHADOW_END: usize = 20;

fn owner() -> Vec<String> {
    ["root", "module", "nested"].map(str::to_owned).to_vec()
}
fn raw(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn declaration_site_and_visibility_updates_are_independently_observable() {
    let mut aliases = Aliases::default();
    let scope = Scope::at(owner(), &[], DECLARATION);
    assert!(aliases.declare(&scope, NAME.into(), Some(raw(&["root"]))));
    assert!(aliases.declare(&scope, NAME.into(), Some(owner())));
    assert!(!aliases.declare(&scope, NAME.into(), Some(owner())));
    assert_eq!(
        aliases.local_declarations(&raw(&[NAME]), &scope),
        BTreeSet::from([DECLARATION])
    );
    assert_eq!(
        aliases.local_declarations(&raw(&["self", NAME]), &scope),
        BTreeSet::from([DECLARATION])
    );
    for path in [
        vec![],
        vec!["self"],
        vec!["crate"],
        vec!["super", "super"],
        vec!["Unknown"],
    ] {
        assert!(
            aliases.local_declarations(&raw(&path), &scope).is_empty(),
            "{path:?}"
        );
    }
    let other = Scope::at(raw(&["different"]), &[], DECLARATION);
    assert!(
        aliases
            .local_declarations(&raw(&["crate", NAME]), &other)
            .is_empty()
    );
}

#[test]
fn ranged_generic_shadow_is_half_open_and_does_not_escape_its_name_or_module() {
    let mut aliases = Aliases::default();
    let declaration = Scope::at(owner(), &[], DECLARATION);
    aliases.declare(&declaration, NAME.into(), None);
    aliases.shadow_range(owner(), NAME.into(), SHADOW_START..SHADOW_END);
    aliases.shadow_range(raw(&["other"]), NAME.into(), 0..SHADOW_END);
    aliases.shadow_range(owner(), "Other".into(), 0..SHADOW_END);
    for (position, shadowed) in [
        (SHADOW_START - 1, false),
        (SHADOW_START, true),
        (SHADOW_END - 1, true),
        (SHADOW_END, false),
    ] {
        let scope = Scope::at(owner(), &[], position);
        assert_eq!(
            aliases.local_declarations(&raw(&[NAME]), &scope).is_empty(),
            shadowed
        );
        assert_eq!(aliases.locally_clean(&raw(&[NAME]), &scope), shadowed);
        assert_eq!(
            aliases
                .local_targets(&raw(&[NAME]), &scope)
                .unwrap()
                .is_empty(),
            shadowed
        );
    }
}

#[test]
fn nested_aliases_and_explicit_shadows_override_outer_declarations() {
    let mut aliases = Aliases::default();
    let outer = Scope::at(owner(), &[], DECLARATION);
    let nested = Scope::at(owner(), &[NESTED_BLOCK], SHADOW_START);
    aliases.declare(&outer, NAME.into(), None);
    aliases.insert_clean(&nested, NAME.into(), raw(&["foreign", "Type"]));
    assert!(
        aliases
            .local_declarations(&raw(&[NAME]), &nested)
            .is_empty()
    );
    assert_eq!(
        aliases.local_targets(&raw(&[NAME, "member"]), &nested),
        Some(vec![raw(&["foreign", "Type", "member"])])
    );
    assert!(aliases.locally_clean(&raw(&[NAME]), &nested));
    assert!(!aliases.locally_clean(&raw(&["self", NAME]), &nested));
    assert!(!aliases.locally_clean(&[], &nested));
    aliases.shadow(&nested, NAME.into());
    assert!(
        aliases
            .local_targets(&raw(&[NAME]), &nested)
            .unwrap()
            .is_empty()
    );
    assert!(
        aliases
            .local_declarations(&raw(&[NAME]), &nested)
            .is_empty()
    );
    assert!(
        aliases
            .local_declarations(&raw(&[NAME]), &outer)
            .contains(&DECLARATION)
    );
}

#[test]
fn binding_site_handles_rooted_paths_without_inventing_missing_targets() {
    for path in [vec![], vec!["self"], vec!["crate"], vec!["super"]] {
        assert!(binding_site(&raw(&path), &owner()).is_none());
    }
    assert!(binding_site(&raw(&["crate", NAME]), &[]).is_none());
    assert_eq!(
        binding_site(&raw(&["super", "super", NAME]), &owner())
            .unwrap()
            .0,
        raw(&["root"])
    );
    assert_eq!(
        binding_site(&raw(&["self", NAME]), &owner()).unwrap().0,
        owner()
    );
    assert_eq!(
        binding_site(&raw(&["crate", NAME]), &owner()).unwrap().0,
        raw(&["root"])
    );
}
