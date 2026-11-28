use crate::support::{Identity, observe};
use yolo_hub_delegation::{FIRST_POLICY_EPOCH, NO_REEXPORTS};
use yolo_hub_federation_directory::{GRANT_LIFETIME_MS, LocalGrant};
use yolo_hub_federation_settings::ExportPolicy;
use yolo_hub_peer_protocol::FIRST_REVISION;

#[test]
fn direct_grants_are_reused_only_for_current_rights_recipient_epoch_and_validity() {
    let signer = Identity::new();
    let first = Identity::new();
    let other = Identity::new();
    let mut policy = ExportPolicy {
        origin: signer.identity.hub(),
        remaining_reexports: NO_REEXPORTS,
        rights: observe(),
    };
    let retained = LocalGrant::default();
    let now = GRANT_LIFETIME_MS;
    let epoch = FIRST_POLICY_EPOCH;
    let first_grant = retained
        .for_peer(&signer, first.identity, epoch, &policy, now)
        .unwrap();
    let within = now + GRANT_LIFETIME_MS / 2 - FIRST_REVISION;
    assert_eq!(
        retained
            .for_peer(&signer, first.identity, epoch, &policy, within)
            .unwrap(),
        first_grant
    );
    let renewed = retained
        .for_peer(
            &signer,
            first.identity,
            epoch,
            &policy,
            within + FIRST_REVISION,
        )
        .unwrap();
    assert_ne!(renewed, first_grant);
    let transferred = retained
        .for_peer(
            &signer,
            other.identity,
            epoch,
            &policy,
            within + FIRST_REVISION,
        )
        .unwrap();
    assert_eq!(transferred.recipient(), other.identity);
    let next_epoch = epoch + FIRST_REVISION;
    let revised = retained
        .for_peer(
            &signer,
            other.identity,
            next_epoch,
            &policy,
            within + FIRST_REVISION,
        )
        .unwrap();
    assert_eq!(revised.policy_epoch(), next_epoch);
    policy.rights = crate::support::rights();
    policy.remaining_reexports = crate::support::REEXPORT_DEPTH;
    let new_policy = retained
        .for_peer(
            &signer,
            other.identity,
            next_epoch,
            &policy,
            within + FIRST_REVISION,
        )
        .unwrap();
    assert_ne!(new_policy, revised);
    let rewound = retained
        .for_peer(&signer, other.identity, next_epoch, &policy, now)
        .unwrap();
    assert_eq!(
        rewound.links()[0].fields().claims.expires_at_ms,
        now + GRANT_LIFETIME_MS
    );
    assert_ne!(rewound, new_policy);
}
