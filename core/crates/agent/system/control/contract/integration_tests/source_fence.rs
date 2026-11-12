use crate::{constants::*, fixture::*};
use yolo_agent_control_contract::*;

#[test]
fn replacing_binding_under_an_older_witness_is_rejected_even_when_every_field_is_well_formed() {
    let original = observation();
    let mut changed = original.binding().clone();
    changed.updated_at_ms += i64::from(STEP);
    changed.cwd = OTHER_CWD.into();
    assert_eq!(
        NativeControlObservation::new(changed.clone(), original.witness().clone(), false),
        Err(ControlContractError::BindingMismatch)
    );
    let mut fields = original.witness().fields().clone();
    fields.binding_digest = binding_digest(&changed).unwrap();
    fields.authority_digest = authority_digest(&(REVISION + TIME_STEP)).unwrap();
    let replacement =
        NativeControlObservation::new(changed, NativeControlWitness::new(fields).unwrap(), false)
            .unwrap();
    assert_eq!(
        replacement.witness().fields().runtime_epoch,
        original.witness().fields().runtime_epoch
    );
    assert_ne!(
        replacement.witness().fields().binding_digest,
        original.witness().fields().binding_digest
    );
    assert_ne!(
        replacement.witness().fields().authority_digest,
        original.witness().fields().authority_digest
    );
}

#[test]
fn exact_paths_and_boot_bytes_survive_the_private_wire_without_filesystem_or_process_side_effects()
{
    let original = observation();
    let wire = serde_json::to_vec(&original).unwrap();
    let decoded: NativeControlObservation = serde_json::from_slice(&wire).unwrap();
    assert_eq!(decoded.binding().cwd, CWD);
    assert_eq!(decoded.witness().fields().boot_id, BOOT_ID);
    let mut normalized = original.binding().clone();
    normalized.cwd = normalized.cwd.trim_end().into();
    assert_ne!(
        binding_digest(&normalized).unwrap(),
        original.witness().fields().binding_digest
    );
}
