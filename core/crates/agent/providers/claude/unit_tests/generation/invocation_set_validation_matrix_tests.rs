use super::super::authority_set::{ActiveBindingPublication, ClaudeInvocationAuthorities};
use super::super::{constants::*, record};
use super::support::Fixture;
use uuid::Uuid;

const FIRST: usize = 0;

#[test]
fn invocation_set_rejects_each_invalid_header_inventory_and_authority_identity() {
    let fixture = Fixture::new();
    let original = ClaudeInvocationAuthorities::new(record::begin(None, &fixture.begin).unwrap());
    original.validate().unwrap();
    let mutations: &[(&str, fn(&mut ClaudeInvocationAuthorities))] = &[
        (stringify!(schema_version), |value| {
            value.schema_version += 1
        }),
        (stringify!(terminal_id), |value| {
            value.terminal_id = Uuid::nil()
        }),
        (stringify!(active_invocation_id), |value| {
            value.active_invocation_id = Uuid::nil()
        }),
        (stringify!(foreign_active), |value| {
            value.active_invocation_id = Uuid::new_v4()
        }),
        (stringify!(publication_generation), |value| {
            value.publication_generation = INITIAL_PUBLICATION_GENERATION - REVISION_INCREMENT
        }),
        (stringify!(empty_inventory), |value| {
            value.authorities.clear()
        }),
        (stringify!(oversized_inventory), |value| {
            value.authorities = std::iter::repeat_n(
                value.authorities[FIRST].clone(),
                MAX_INVOCATION_AUTHORITIES + 1,
            )
            .collect()
        }),
        (stringify!(authority_session), |value| {
            value.authorities[FIRST].terminal_id = Uuid::new_v4()
        }),
        (stringify!(authority_schema), |value| {
            value.authorities[FIRST].schema_version += 1
        }),
        (stringify!(duplicate_invocation), |value| {
            value
                .authorities
                .push_back(value.authorities[FIRST].clone())
        }),
        (stringify!(unpublished_binding), |value| {
            value.authorities[FIRST].binding_revision = Some(value.authorities[FIRST].revision)
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            AUTHORITY_SET_INVALID,
            "{field}"
        );
    }
}

#[test]
fn binding_publication_requires_matching_active_invocation_generation_and_revision() {
    let fixture = Fixture::new();
    let mut original =
        ClaudeInvocationAuthorities::new(record::begin(None, &fixture.begin).unwrap());
    let revision = original.authorities[FIRST].revision;
    original.authorities[FIRST].binding_revision = Some(revision);
    original.binding_publication = Some(ActiveBindingPublication {
        invocation_id: original.active_invocation_id,
        publication_generation: original.publication_generation,
        authority_revision: revision,
    });
    original.validate().unwrap();
    let mutations: &[(&str, fn(&mut ClaudeInvocationAuthorities))] = &[
        (stringify!(invocation_id), |value| {
            value.binding_publication.as_mut().unwrap().invocation_id = Uuid::new_v4()
        }),
        (stringify!(publication_generation), |value| {
            value
                .binding_publication
                .as_mut()
                .unwrap()
                .publication_generation += REVISION_INCREMENT
        }),
        (stringify!(authority_revision), |value| {
            value
                .binding_publication
                .as_mut()
                .unwrap()
                .authority_revision += REVISION_INCREMENT
        }),
        (stringify!(binding_revision), |value| {
            value.authorities[FIRST].binding_revision = None
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            AUTHORITY_SET_INVALID,
            "{field}"
        );
    }
}
