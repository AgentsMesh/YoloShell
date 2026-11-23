use super::*;

#[test]
fn both_platform_proofs_roundtrip_without_live_device_or_mount_fields() {
    for identity in [fixture::linux(), fixture::macos()] {
        let wire = serde_json::to_value(identity).unwrap();
        assert_eq!(
            serde_json::from_value::<DurableObjectIdentity>(wire.clone()).unwrap(),
            identity
        );
        let text = serde_json::to_string(&wire).unwrap();
        assert!(!text.contains("device"));
        assert!(!text.contains("mount_id"));
    }
}

#[test]
fn wire_rejects_unknown_missing_foreign_and_retired_shapes() {
    let valid = serde_json::to_value(fixture::linux()).unwrap();
    let mut cases = Vec::new();
    let mut changed = valid.clone();
    changed["schema"] = serde_json::json!(u16::MAX);
    cases.push(changed);
    let mut changed = valid.clone();
    changed["extra"] = serde_json::Value::Null;
    cases.push(changed);
    let mut changed = valid.clone();
    changed["proof"] = serde_json::json!({"unknown_platform": changed["proof"]["linux"].clone()});
    cases.push(changed);
    let mut changed = valid.clone();
    changed["proof"]["linux"]["filesystem_uuid"] =
        serde_json::to_value([u8::default(); UUID_BYTES]).unwrap();
    cases.push(changed);
    let mut changed = valid.clone();
    changed["proof"]["linux"]["extra"] = serde_json::Value::Null;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.as_object_mut().unwrap().remove("birth");
    cases.push(changed);
    cases.push(
        serde_json::json!({"device": 41, "inode": 251, "birth": {"seconds": 0, "nanoseconds": 0}}),
    );
    for wire in cases {
        assert!(serde_json::from_value::<DurableObjectIdentity>(wire).is_err());
    }
}

#[test]
fn all_proof_fields_participate_in_identity_equality() {
    let baseline = fixture::linux();
    let mut record = baseline.record;
    record.kind = ObjectKind::RegularFile;
    assert_ne!(DurableObjectIdentity::try_from(record).unwrap(), baseline);
    record = baseline.record;
    record.birth.nanoseconds += 1;
    assert_ne!(DurableObjectIdentity::try_from(record).unwrap(), baseline);
    record = baseline.record;
    if let Proof::Linux {
        filesystem_uuid, ..
    } = &mut record.proof
    {
        filesystem_uuid[0] ^= 1;
    }
    assert_ne!(DurableObjectIdentity::try_from(record).unwrap(), baseline);
    assert_ne!(fixture::macos(), baseline);
}
