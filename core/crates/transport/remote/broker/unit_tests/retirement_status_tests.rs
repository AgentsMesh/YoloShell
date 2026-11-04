use super::*;
use crate::remote_broker::constants::WORKING_DIRECTORY_CAPABILITY;
use uuid::Uuid;

const HOST_ID: &str = "retirement-host";
const REVISION: &str = "retirement-revision";

fn status() -> BrokerStatus {
    BrokerStatus::disconnected(Uuid::nil(), HOST_ID.into(), REVISION.into(), u64::default())
}

#[test]
fn retirement_status_requires_host_identity_and_capability() {
    let current = status();
    validate_retirement_status(&current, HOST_ID).unwrap();
    assert!(validate_retirement_status(&current, REVISION).is_err());

    let mut legacy = current;
    legacy.capabilities = vec![WORKING_DIRECTORY_CAPABILITY.into()];
    assert!(validate_retirement_status(&legacy, HOST_ID).is_err());
}
