use super::fixtures::{self, constants::*};
use crate::{
    writer_conversion::{internal_grant, public_grant, runtime},
    writer_observation::observation,
};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn conversion_preserves_exact_runtime_controller_generation_and_secret() {
    let raw = fixtures::grant();
    let converted = public_grant(fixtures::target(), fixtures::principal(), raw.clone()).unwrap();
    assert_eq!(converted.fields().target, fixtures::target());
    assert_eq!(internal_grant(converted).unwrap(), raw);
    let target = runtime(fixtures::target());
    assert_eq!(
        target.terminal_id,
        fixtures::target().address.terminal_id.as_uuid()
    );
    assert_eq!(
        target.runtime_epoch,
        fixtures::target().runtime_epoch.as_uuid()
    );
}

#[test]
fn holder_mismatch_and_malformed_remote_lease_are_rejected() {
    let mutations: &[fn(&mut yolo_runtime::model::WriterLeaseGrant)] = &[
        |g| g.lease.holder_id = OTHER_HOLDER.into(),
        |g| g.lease.controller_instance_id = None,
        |g| g.lease.controller_instance_id = Some(OTHER_HOLDER.into()),
        |g| g.lease.generation = 0,
    ];
    for mutate in mutations {
        let mut raw = fixtures::grant();
        mutate(&mut raw);
        assert_eq!(
            public_grant(fixtures::target(), fixtures::principal(), raw),
            Err(PublicError::InvalidLease)
        );
    }
}

#[test]
fn read_only_observation_preserves_non_uuid_existing_clients_without_secrets() {
    let mut raw = fixtures::grant().lease;
    raw.holder_id = OTHER_HOLDER.into();
    raw.controller_instance_id = None;
    let observed = observation(fixtures::target(), raw.clone()).unwrap();
    assert_eq!(observed.fields().holder_id, OTHER_HOLDER);
    assert_eq!(observed.fields().generation, raw.generation);
    raw.generation = 0;
    assert_eq!(
        observation(fixtures::target(), raw),
        Err(PublicError::InvalidLease)
    );
}
