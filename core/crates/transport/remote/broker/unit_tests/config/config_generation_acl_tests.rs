use super::tests::Fixture;
use super::*;

#[test]
fn persisted_generation_rejects_directory_and_file_grant_acls() {
    let fixture = Fixture::new();
    let generation = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    let directory = generation.config_path().parent().unwrap();
    crate::acl_fixture::add_read_grant(directory);
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());

    let fixture = Fixture::new();
    let generation = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    crate::acl_fixture::add_read_grant(generation.config_path());
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());
}
