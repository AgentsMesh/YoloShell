use yolo_terminal_model::{FaultReapProgress, FaultReapProgressKind, FaultReapStatus};

const FIRST_PROGRESS_SEQUENCE: u64 = 1;
const OBSERVED_AT_MS: i64 = 1_234;
const EMPTY_FAULT: &str = "";
const PADDED_FAULT: &str = " cleanup fault with surrounding whitespace ";
const TEST_FAULT: &str = "cleanup fault";
const UNKNOWN_FIELD: &str = "unknown";

fn progress(kind: FaultReapProgressKind) -> FaultReapProgress {
    FaultReapProgress {
        sequence: FIRST_PROGRESS_SEQUENCE,
        observed_at_ms: OBSERVED_AT_MS,
        kind,
    }
}

#[test]
fn status_round_trips_every_progress_kind() {
    for kind in [
        FaultReapProgressKind::Handoff,
        FaultReapProgressKind::CleanupAdvanced,
        FaultReapProgressKind::ObserverOnly,
        FaultReapProgressKind::Completed,
    ] {
        let status = FaultReapStatus {
            custody_retained: true,
            observer_only: kind == FaultReapProgressKind::ObserverOnly,
            last_semantic_progress: Some(progress(kind)),
            last_fault: Some(TEST_FAULT.to_owned()),
        };
        let encoded = serde_json::to_value(&status).unwrap();
        let decoded: FaultReapStatus = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, status);
    }
}

#[test]
fn optional_details_are_omitted_and_default_when_absent() {
    let status = FaultReapStatus {
        custody_retained: false,
        observer_only: false,
        last_semantic_progress: None,
        last_fault: None,
    };
    let encoded = serde_json::to_value(&status).unwrap();
    assert!(encoded.get(stringify!(last_semantic_progress)).is_none());
    assert!(encoded.get(stringify!(last_fault)).is_none());
    let decoded: FaultReapStatus = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, status);
}

#[test]
fn fault_detail_preserves_arbitrary_diagnostic_text() {
    for detail in [EMPTY_FAULT, PADDED_FAULT] {
        let status = FaultReapStatus {
            custody_retained: true,
            observer_only: false,
            last_semantic_progress: None,
            last_fault: Some(detail.to_owned()),
        };
        let encoded = serde_json::to_value(&status).unwrap();
        let decoded: FaultReapStatus = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.last_fault.as_deref(), Some(detail));
    }
}

#[test]
fn optional_details_reject_explicit_null() {
    let status = FaultReapStatus {
        custody_retained: true,
        observer_only: false,
        last_semantic_progress: Some(progress(FaultReapProgressKind::CleanupAdvanced)),
        last_fault: Some(TEST_FAULT.to_owned()),
    };
    for field in [stringify!(last_semantic_progress), stringify!(last_fault)] {
        let mut encoded = serde_json::to_value(&status).unwrap();
        encoded[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<FaultReapStatus>(encoded).is_err());
    }
}

#[test]
fn status_and_progress_reject_unknown_fields() {
    let mut status = serde_json::to_value(FaultReapStatus {
        custody_retained: true,
        observer_only: false,
        last_semantic_progress: Some(progress(FaultReapProgressKind::CleanupAdvanced)),
        last_fault: None,
    })
    .unwrap();
    status[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<FaultReapStatus>(status).is_err());

    let mut nested = serde_json::to_value(progress(FaultReapProgressKind::Handoff)).unwrap();
    nested[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<FaultReapProgress>(nested).is_err());
}
