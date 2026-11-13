#[path = "constants.rs"]
mod constants;
use anyhow::{Context, Result, ensure};
use constants::*;
use yolo_broker_launch_authority::HubStartAuthorityGuard;
use yolo_remote_broker::{BROKER_PROTOCOL_VERSION, BrokerStatus, HUB_TRANSPORT_CAPABILITY};

fn main() -> Result<()> {
    let mode = std::env::args().nth(EXPECTED_STAGES).unwrap();
    let (guard, credentials) =
        HubStartAuthorityGuard::from_stdio().context(CHILD_RECEIVE_CONTEXT)?;
    guard.verify().context(CHILD_VERIFY_CONTEXT)?;
    if mode == CREDENTIAL {
        ensure!(guard.has_credentials(), "credential envelope was absent");
        let encoded = credentials.encode()?;
        let value: serde_json::Value = yolo_capnp_codec::decode(&encoded)?;
        ensure!(
            value[PASSPHRASES_FIELD][0].as_str() == Some(EXPECTED_SECRET),
            "credential envelope differs"
        );
    } else {
        ensure!(!guard.has_credentials(), "unexpected credential envelope");
    }
    let mut ready = BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: false,
        instance_id: uuid::Uuid::new_v4(),
        host_id: guard.scope().profile.profile_id().into(),
        connection_id: Some(uuid::Uuid::new_v4()),
        connection_generation: GENERATION,
        config_revision: guard.scope().config_revision.clone(),
        capabilities: vec![HUB_TRANSPORT_CAPABILITY.into()],
    };
    guard.commit(&ready).context(CHILD_COMMIT_CONTEXT)?;
    if mode == ABANDON {
        return Ok(());
    }
    if mode == REPLAY {
        ensure!(
            guard.commit(&ready).is_err(),
            "handoff authority was reusable"
        );
        return Ok(());
    }
    ready.adopted = true;
    guard.finished(&ready).context(CHILD_FINISHED_CONTEXT)?;
    ensure!(guard.verify().is_err(), "completed authority remained live");
    println!("{}", serde_json::to_string(&ready)?);
    Ok(())
}
