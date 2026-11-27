use super::*;

const ROUTE_REVISION_INCREMENT: u64 = 1;

#[test]
fn explicit_revocation_is_durable_and_idempotent_for_confirmed_exit() {
    let fixture = Fixture::new();
    let mut exited = fixture.verified_authority(Generation::INITIAL.0);
    exited.binding.state = AgentState::Exited;
    exited.binding.process_pid = None;
    exited.binding.process_start_time = None;
    fixture.publish(&exited).unwrap();
    let (_, identity) = fixture
        .namespace
        .read_json_with_identity::<CodexBindingAuthority>(fixture.store().path())
        .unwrap();

    fixture
        .store()
        .revoke_exact_in_namespace_with_commit_fence(&fixture.namespace, &exited, identity, || {
            Ok(())
        })
        .unwrap();
    fixture
        .store()
        .revoke_exact_in_namespace_with_commit_fence(&fixture.namespace, &exited, identity, || {
            Ok(())
        })
        .unwrap();

    let revoked = fixture
        .store()
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    assert_eq!(revoked, exited.revoked().unwrap());
    assert_eq!(revoked.status, CodexAuthorityStatus::Revoked);
}

#[test]
fn revocation_rejects_replacement_but_allows_a_new_route_afterward() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let (_, identity) = fixture
        .namespace
        .read_json_with_identity::<CodexBindingAuthority>(fixture.store().path())
        .unwrap();
    let mut replacement = active.clone();
    replacement.version.route_revision += ROUTE_REVISION_INCREMENT;
    fixture
        .namespace
        .write_json(fixture.store().path(), &replacement)
        .unwrap();

    assert!(
        fixture
            .store()
            .revoke_exact_in_namespace_with_commit_fence(
                &fixture.namespace,
                &active,
                identity,
                || Ok(()),
            )
            .is_err()
    );
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(replacement.clone())
    );

    let (_, replacement_identity) = fixture
        .namespace
        .read_json_with_identity::<CodexBindingAuthority>(fixture.store().path())
        .unwrap();
    fixture
        .store()
        .revoke_exact_in_namespace_with_commit_fence(
            &fixture.namespace,
            &replacement,
            replacement_identity,
            || Ok(()),
        )
        .unwrap();
    let mut next = active;
    next.version.route_revision = replacement.version.route_revision + ROUTE_REVISION_INCREMENT;
    fixture.publish(&next).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(next)
    );
}

#[test]
fn revoked_route_rejects_every_later_publication_until_a_new_route() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let revoked = active.revoked().unwrap();
    fixture.publish(&revoked).unwrap();

    let mut later_generation = active.clone();
    later_generation.version.generation = Generation(active.version.generation.0 + 1);
    assert!(fixture.publish(&later_generation).is_err());

    let mut later_revision = active.clone();
    later_revision.version.authority_revision += ROUTE_REVISION_INCREMENT;
    assert!(fixture.publish(&later_revision).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(revoked)
    );

    let mut next_route = active;
    next_route.version.route_revision += ROUTE_REVISION_INCREMENT;
    fixture.publish(&next_route).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(next_route)
    );
}

#[test]
fn captured_authority_deletion_is_replaced_by_the_exact_revocation() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let (_, identity) = fixture
        .namespace
        .read_json_with_identity::<CodexBindingAuthority>(fixture.store().path())
        .unwrap();
    fixture
        .namespace
        .remove_file(fixture.store().path())
        .unwrap();

    fixture
        .store()
        .revoke_exact_in_namespace_with_commit_fence(&fixture.namespace, &active, identity, || {
            Ok(())
        })
        .unwrap();

    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(active.revoked().unwrap())
    );
}
