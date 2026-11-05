use super::*;

const TEST_READER_HANDLE: u64 = 70_001;
const TEST_WRITER_HANDLE: u64 = 70_002;

#[test]
fn macos_attestation_requires_two_distinct_nonzero_handles() {
    assert!(
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE,)
            .is_well_formed()
    );
    assert!(
        !CustodyDescriptorSet::macos_process_capability(INVALID_PIPE_HANDLE, TEST_WRITER_HANDLE,)
            .is_well_formed()
    );
    assert!(
        !CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, INVALID_PIPE_HANDLE,)
            .is_well_formed()
    );
    assert!(
        !CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_READER_HANDLE,)
            .is_well_formed()
    );
}

#[test]
fn descriptor_set_wire_rejects_unknown_attestation_fields() {
    let descriptor_set =
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE);
    let mut value = serde_json::to_value(descriptor_set).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_owned(), serde_json::Value::Bool(true));

    assert!(serde_json::from_value::<CustodyDescriptorSet>(value).is_err());
}
