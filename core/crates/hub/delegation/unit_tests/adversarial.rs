use crate::support::*;
use yolo_hub_delegation::*;
use yolo_hub_identity::HubId;

#[test]
fn empty_cyclic_and_unrooted_chains_are_not_authority() {
    let (origin, relay, recipient) = (Identity::new(), Identity::new(), Identity::new());
    assert_eq!(DelegationChain::new(Vec::new()), Err(DelegationError::Path));
    let root = SignedGrant::issue(root_claims(&origin, &relay), &origin).unwrap();
    let child = SignedGrant::issue(child_claims(&root, &recipient), &relay).unwrap();
    assert_eq!(
        DelegationChain::new(vec![child.clone()]),
        Err(DelegationError::Origin)
    );
    let chain = DelegationChain::new(vec![root.clone(), child.clone()]).unwrap();
    let cycle = SignedGrant::issue(child_claims(&child, &relay), &recipient).unwrap();
    assert_eq!(chain.append(cycle), Err(DelegationError::Path));
    let mut unrelated = child_claims(&root, &recipient);
    unrelated.origin = recipient.identity;
    assert!(SignedGrant::issue(unrelated, &relay).is_err());
    let mut false_root = root_claims(&origin, &relay);
    false_root.parent = Some(root.fingerprint().unwrap());
    assert_eq!(
        DelegationChain::new(vec![SignedGrant::issue(false_root, &origin).unwrap()]),
        Err(DelegationError::Origin)
    );
}

#[test]
fn no_reexport_budget_cannot_be_replenished_by_the_relay() {
    let (origin, relay, recipient) = (Identity::new(), Identity::new(), Identity::new());
    let mut claims = root_claims(&origin, &relay);
    claims.remaining_reexports = NO_REEXPORTS;
    let root = SignedGrant::issue(claims, &origin).unwrap();
    let child = SignedGrant::issue(child_claims(&root, &recipient), &relay).unwrap();
    assert_eq!(
        DelegationChain::new(vec![root, child]),
        Err(DelegationError::Escalation)
    );
}

#[test]
fn claim_bounds_and_identity_wire_are_enforced() {
    let (origin, recipient) = (Identity::new(), Identity::new());
    let original = root_claims(&origin, &recipient);
    for field in 0..7 {
        let mut claims = original.clone();
        match field {
            0 => claims.grant_id = uuid::Uuid::nil(),
            1 => claims.recipient = origin.identity,
            2 => claims.policy_epoch = 0,
            3 => claims.expires_at_ms = claims.not_before_ms,
            4 => claims.expires_at_ms = claims.not_before_ms + MAX_GRANT_LIFETIME_MS + STEP,
            5 => claims.remaining_reexports = MAX_DELEGATION_DEPTH + 1,
            _ => claims.rights = Default::default(),
        }
        assert_eq!(claims.validate(), Err(DelegationError::Claims));
    }
    assert_eq!(
        PeerIdentity::new(HubId::new(), [0; ED25519_PUBLIC_KEY_BYTES]),
        Err(DelegationError::Identity)
    );
    let fields: IdentityFields = origin.identity.into();
    assert_eq!(PeerIdentity::try_from(fields).unwrap(), origin.identity);
    assert_eq!(
        serde_json::from_slice::<PeerIdentity>(&serde_json::to_vec(&origin.identity).unwrap())
            .unwrap(),
        origin.identity
    );
}
