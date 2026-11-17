use crate::support::*;
use yolo_hub_delegation::*;
use yolo_hub_routing::Right;

#[test]
fn origin_to_relay_to_entry_reduces_rights_and_keeps_original_issuer() {
    let (origin, relay, entry) = (Identity::new(), Identity::new(), Identity::new());
    let root = SignedGrant::issue(root_claims(&origin, &relay), &origin).unwrap();
    let mut claims = child_claims(&root, &entry);
    claims.rights = observe();
    let delegated = SignedGrant::issue(claims, &relay).unwrap();
    let chain = DelegationChain::new(vec![root])
        .unwrap()
        .append(delegated)
        .unwrap();
    assert_eq!(chain.origin(), origin.identity);
    assert_eq!(chain.recipient(), entry.identity);
    assert_eq!(chain.links().len(), 2);
    chain
        .authorize(
            origin.identity,
            FIRST_POLICY_EPOCH,
            entry.identity,
            NOW,
            &observe(),
        )
        .unwrap();
    assert_eq!(
        chain.authorize(
            origin.identity,
            FIRST_POLICY_EPOCH,
            entry.identity,
            NOW,
            &rights()
        ),
        Err(DelegationError::Escalation)
    );
    assert_eq!(
        chain.authorize(
            origin.identity,
            FIRST_POLICY_EPOCH,
            entry.identity,
            EXPIRES,
            &observe()
        ),
        Err(DelegationError::Expired)
    );
    assert_eq!(
        chain.authorize(
            origin.identity,
            FIRST_POLICY_EPOCH,
            entry.identity,
            BEFORE - STEP,
            &observe()
        ),
        Err(DelegationError::Expired)
    );
    assert_eq!(
        chain.authorize(
            origin.identity,
            FIRST_POLICY_EPOCH + STEP,
            entry.identity,
            NOW,
            &observe()
        ),
        Err(DelegationError::Revoked)
    );
    assert_eq!(
        chain.authorize(
            relay.identity,
            FIRST_POLICY_EPOCH,
            entry.identity,
            NOW,
            &observe()
        ),
        Err(DelegationError::Origin)
    );
    assert_eq!(
        chain.authorize(
            origin.identity,
            FIRST_POLICY_EPOCH,
            relay.identity,
            NOW,
            &observe()
        ),
        Err(DelegationError::Recipient)
    );
    let encoded = serde_json::to_vec(&chain).unwrap();
    assert_eq!(
        serde_json::from_slice::<DelegationChain>(&encoded).unwrap(),
        chain
    );
    let links: Vec<SignedGrant> = chain.into();
    assert_eq!(links.len(), 2);
}

#[test]
fn each_attempted_authority_expansion_is_rejected_even_with_a_valid_signature() {
    let (origin, relay, entry) = (Identity::new(), Identity::new(), Identity::new());
    let root = SignedGrant::issue(root_claims(&origin, &relay), &origin).unwrap();
    let chain = DelegationChain::new(vec![root.clone()]).unwrap();
    let original = child_claims(&root, &entry);
    for field in 0..6 {
        let mut claims = original.clone();
        match field {
            0 => claims.rights = [Right::FileRead].into_iter().collect(),
            1 => claims.not_before_ms -= STEP,
            2 => claims.expires_at_ms += STEP,
            3 => claims.policy_epoch += STEP,
            4 => claims.remaining_reexports += 1,
            _ => claims.parent = None,
        }
        let signed = SignedGrant::issue(claims, &relay).unwrap();
        assert!(chain.append(signed).is_err());
    }
}

#[test]
fn signature_substitution_and_untrusted_identity_never_authorize_data() {
    let (origin, recipient, intruder) = (Identity::new(), Identity::new(), Identity::new());
    let original = root_claims(&origin, &recipient);
    assert_eq!(
        SignedGrant::issue(original.clone(), &intruder),
        Err(DelegationError::Identity)
    );
    let grant = SignedGrant::issue(original, &origin).unwrap();
    let mut fields = grant.fields().clone();
    fields.claims.recipient = intruder.identity;
    assert_eq!(
        SignedGrant::try_from(fields),
        Err(DelegationError::Signature)
    );
    let mut fields = grant.fields().clone();
    fields.signature.pop();
    assert_eq!(
        SignedGrant::try_from(fields),
        Err(DelegationError::Signature)
    );
    let mut fields = grant.fields().clone();
    fields.signature[0] ^= u8::MAX;
    assert_eq!(
        SignedGrant::try_from(fields),
        Err(DelegationError::Signature)
    );
    let mut value = serde_json::to_value(grant).unwrap();
    value[INJECTED] = true.into();
    assert!(serde_json::from_value::<SignedGrant>(value).is_err());
}
