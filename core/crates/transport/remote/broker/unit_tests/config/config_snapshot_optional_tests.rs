use super::*;

#[test]
fn optional_artifact_and_identity_files_can_be_absent() {
    let fixture = Fixture::new();
    let mut value = fixture.value();
    value.as_object_mut().unwrap().remove(ARTIFACT_CONFIG_FIELD);
    value[IDENTITIES_CONFIG_FIELD] = Value::Array(Vec::new());
    fixture.write_value(&value);

    let staged = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    let parent = staged.config_path().parent().unwrap();
    for name in [GENERATION_ARTIFACT_FILE.to_owned(), identity_name(0)] {
        assert!(!parent.join(name).exists());
    }
}
