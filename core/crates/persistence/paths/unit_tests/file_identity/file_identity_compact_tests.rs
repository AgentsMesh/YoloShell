use super::*;
use serde::Serialize;

const DEVICE: u64 = 3;
const INODE: u64 = 5;
const LENGTH: u64 = 7;
const MODIFIED_SECONDS: i64 = 11;
const MODIFIED_NANOSECONDS: i64 = 13;
const CHANGED_SECONDS: i64 = 17;
const CHANGED_NANOSECONDS: i64 = 19;
const BIRTH_SECONDS: i64 = 23;
const BIRTH_NANOSECONDS: i64 = 29;
const NEGATIVE_NANOSECONDS: i64 = -1;
const NONCANONICAL_PADDING: char = '=';

#[derive(Deserialize, Serialize)]
struct Wire(#[serde(with = "super")] FileIdentity);

#[test]
fn exact_identity_round_trips_through_canonical_base64url() {
    let identity = identity();
    let encoded = serde_json::to_value(Wire(identity)).unwrap();
    assert!(encoded.is_string());
    assert_eq!(serde_json::from_value::<Wire>(encoded).unwrap().0, identity);
}

#[test]
fn malformed_and_noncanonical_encodings_are_rejected() {
    let encoded = serde_json::to_value(Wire(identity())).unwrap();
    let mut truncated = encoded.as_str().unwrap().to_owned();
    truncated.pop().unwrap();
    assert!(serde_json::from_value::<Wire>(truncated.into()).is_err());

    let mut padded = encoded.as_str().unwrap().to_owned();
    padded.push(NONCANONICAL_PADDING);
    assert!(serde_json::from_value::<Wire>(padded.into()).is_err());
}

#[test]
fn out_of_range_timestamps_are_rejected() {
    let upper_bound = i64::try_from(crate::runtime_constants::NANOSECONDS_PER_SECOND).unwrap();
    for nanoseconds in [NEGATIVE_NANOSECONDS, upper_bound] {
        let invalid = FileIdentity {
            modified: FileTimestamp::new(MODIFIED_SECONDS, nanoseconds),
            ..identity()
        };
        let encoded = serde_json::to_value(Wire(invalid)).unwrap();
        assert!(serde_json::from_value::<Wire>(encoded).is_err());
    }
}

#[test]
fn largest_identity_round_trips_exactly() {
    let identity = largest_for_test();
    let encoded = serde_json::to_value(Wire(identity)).unwrap();
    assert_eq!(serde_json::from_value::<Wire>(encoded).unwrap().0, identity);
}

fn identity() -> FileIdentity {
    FileIdentity {
        device: DEVICE,
        inode: INODE,
        length: LENGTH,
        modified: FileTimestamp::new(MODIFIED_SECONDS, MODIFIED_NANOSECONDS),
        changed: FileTimestamp::new(CHANGED_SECONDS, CHANGED_NANOSECONDS),
        birth: FileTimestamp::new(BIRTH_SECONDS, BIRTH_NANOSECONDS),
    }
}
