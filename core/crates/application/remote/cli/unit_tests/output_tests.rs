use super::*;

const TEST_INSTANCE_ID: u128 = 37;
const TEST_CONNECTION_ID: u128 = 41;
const TEST_CONNECTION_GENERATION: u64 = 3;

#[test]
fn start_and_bootstrap_output_only_the_strict_broker_binding() {
    let status = status();
    let expected = identity_value(&status);
    let bootstrap = BootstrapOutput {
        bootstrap: BrokerIdentityOutput::from_status(status.clone()).unwrap(),
    };
    let start = StartOutput {
        start: BrokerIdentityOutput::from_status(status).unwrap(),
    };

    assert_eq!(
        written_value(&bootstrap),
        serde_json::json!({
            (yolo_runtime::cli::constants::BOOTSTRAP_COMMAND): expected.clone()
        })
    );
    assert_eq!(
        written_value(&start),
        serde_json::json!({
            (yolo_runtime::cli::constants::START_COMMAND): expected
        })
    );
}

fn written_value(value: &impl Serialize) -> serde_json::Value {
    let mut bytes = Vec::new();
    write_output(&mut bytes, value).unwrap();
    yolo_capnp_codec::decode(&bytes).unwrap()
}

#[test]
fn output_rejects_an_absent_or_zero_connection_binding() {
    let mut missing = status();
    missing.connection_id = None;
    assert!(BrokerIdentityOutput::from_status(missing).is_err());
    let mut zero = status();
    zero.connection_generation = u64::default();
    assert!(BrokerIdentityOutput::from_status(zero).is_err());
}

fn status() -> BrokerStatus {
    BrokerStatus {
        protocol_version: yolo_remote_broker::BROKER_PROTOCOL_VERSION,
        adopted: true,
        instance_id: uuid::Uuid::from_u128(TEST_INSTANCE_ID),
        host_id: crate::fixture_constants::TEST_HOST_ID.into(),
        connection_id: Some(uuid::Uuid::from_u128(TEST_CONNECTION_ID)),
        connection_generation: TEST_CONNECTION_GENERATION,
        config_revision: crate::fixture_constants::TEST_CONFIG_REVISION.into(),
        capabilities: Vec::new(),
    }
}

fn identity_value(status: &BrokerStatus) -> serde_json::Value {
    serde_json::json!({
        (crate::fixture_constants::PROTOCOL_VERSION_FIELD): status.protocol_version,
        (crate::fixture_constants::INSTANCE_ID_FIELD): status.instance_id,
        (crate::fixture_constants::HOST_ID_FIELD): status.host_id,
        (crate::fixture_constants::CONFIG_REVISION_FIELD): status.config_revision,
        (crate::fixture_constants::CONNECTION_ID_FIELD): status.connection_id,
        (crate::fixture_constants::CONNECTION_GENERATION_FIELD): status.connection_generation,
    })
}
