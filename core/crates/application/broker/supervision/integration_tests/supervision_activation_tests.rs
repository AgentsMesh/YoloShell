use super::*;
use crate::supervision_frame::MAX_FRAME_BYTES;
use std::time::{Duration, Instant};

const TEST_TIMEOUT: Duration = Duration::from_secs(1);

fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}

#[test]
fn readiness_requires_a_complete_capnp_frame() {
    let (reader, mut writer) = BlockingLocalUnixStream::pair().unwrap();
    let frame = yolo_capnp_codec::encode(&uuid::Uuid::new_v4()).unwrap();
    writer
        .write_all(&u32::try_from(frame.len()).unwrap().to_be_bytes())
        .unwrap();
    writer.write_all(&frame[..frame.len() - 1]).unwrap();
    drop(writer);
    assert!(ActivationSender(reader).read_readiness(deadline()).is_err());
}

#[test]
fn readiness_rejects_oversized_header_before_receiving_the_body() {
    let (reader, mut writer) = BlockingLocalUnixStream::pair().unwrap();
    writer
        .write_all(&u32::try_from(MAX_FRAME_BYTES + 1).unwrap().to_be_bytes())
        .unwrap();
    assert!(ActivationSender(reader).read_readiness(deadline()).is_err());
}

#[test]
fn trailing_readiness_bytes_cannot_be_consumed_as_an_adoption_ack() {
    let readiness = BrokerReadiness {
        instance_id: uuid::Uuid::new_v4(),
        host_id: crate::fixture_constants::TEST_HOST_ID.into(),
        config_revision: crate::fixture_constants::TEST_PROFILE_AUTHORITY_SHA256.into(),
    };
    let (reader, mut writer) = BlockingLocalUnixStream::pair().unwrap();
    frame::write(&mut writer, &readiness).unwrap();
    writer.write_all(&[b'x']).unwrap();
    drop(writer);
    let mut sender = ActivationSender(reader);
    sender.read_readiness(deadline()).unwrap();
    assert!(
        super::super::supervision_ack::read_prepared(
            &mut sender.0,
            uuid::Uuid::new_v4(),
            readiness.instance_id,
            deadline(),
        )
        .is_err()
    );
}
