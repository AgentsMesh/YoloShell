use super::*;
use crate::process::constants::{PROCESS_DISCOVERY_CAPABILITY_ENV, PTY_SESSION_CAPABILITY_ENV};

const CAPABILITY: &str = "exact-capability";
const EFFECTIVE_UID: u32 = 42;
const ENV_PREFIX: &[u8] = b"YOLO_PTY_SESSION_CAPABILITY=exact-capability-prefix\0";
const ENV_SUFFIX: &[u8] = b"YOLO_PTY_SESSION_CAPABILITY=exact-capability-suffix\0";
const ENV_EXACT: &[u8] = b"A=B\0YOLO_PTY_SESSION_CAPABILITY=exact-capability\0C=D\0";
const ENV_UNTERMINATED: &[u8] = b"YOLO_PTY_SESSION_CAPABILITY=exact-capability";
const STATUS: &[u8] = b"Name:\ttest\nUid:\t7\t42\t43\t44\n";

#[test]
fn environment_capability_requires_an_exact_nul_field() {
    assert!(matches!(
        parse_capability(ENV_EXACT, PTY_SESSION_CAPABILITY_ENV, CAPABILITY),
        QueryValue::Value(true)
    ));
    assert!(matches!(
        parse_capability(ENV_PREFIX, PTY_SESSION_CAPABILITY_ENV, CAPABILITY),
        QueryValue::Value(false)
    ));
    assert!(matches!(
        parse_capability(ENV_SUFFIX, PTY_SESSION_CAPABILITY_ENV, CAPABILITY),
        QueryValue::Value(false)
    ));
    assert!(matches!(
        parse_capability(
            b"YOLO_PTY_SESSION_CAPABILITY=other\0",
            PTY_SESSION_CAPABILITY_ENV,
            CAPABILITY,
        ),
        QueryValue::Value(false)
    ));
    assert!(matches!(
        parse_capability(ENV_UNTERMINATED, PTY_SESSION_CAPABILITY_ENV, CAPABILITY),
        QueryValue::Pending(CandidatePending::Query(QueryKind::Environment))
    ));
    assert!(matches!(
        parse_capability(b"", PTY_SESSION_CAPABILITY_ENV, CAPABILITY),
        QueryValue::Value(false)
    ));
}

#[test]
fn process_discovery_capability_uses_its_distinct_environment_name() {
    let discovery = format!("{PROCESS_DISCOVERY_CAPABILITY_ENV}={CAPABILITY}\0");
    assert!(matches!(
        parse_capability(
            discovery.as_bytes(),
            PROCESS_DISCOVERY_CAPABILITY_ENV,
            CAPABILITY,
        ),
        QueryValue::Value(true)
    ));
    assert!(matches!(
        parse_capability(discovery.as_bytes(), PTY_SESSION_CAPABILITY_ENV, CAPABILITY,),
        QueryValue::Value(false)
    ));
}

#[test]
fn status_parser_selects_the_effective_uid() {
    assert_eq!(parse_effective_uid(STATUS), Some(EFFECTIVE_UID));
    assert_eq!(parse_effective_uid(b"Uid:\t7\n"), None);
    assert_eq!(parse_effective_uid(b"Uid:\t7\tinvalid\n"), None);
    assert_eq!(parse_effective_uid(b"Name:\ttest\n"), None);
}

#[test]
fn bounded_reader_distinguishes_exact_and_exceeded_limits() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bounded");
    std::fs::write(&path, b"four").unwrap();

    assert!(matches!(
        read_bounded(path.clone(), 4),
        Ok(ReadResult::Complete(_))
    ));
    assert!(matches!(read_bounded(path, 3), Ok(ReadResult::Exceeded(4))));
}

#[test]
fn permission_failures_remain_typed() {
    let denied = std::io::Error::from_raw_os_error(libc::EACCES);
    let missing = std::io::Error::from_raw_os_error(libc::ENOENT);

    assert_eq!(
        io_pending(QueryKind::Environment, &denied),
        CandidatePending::Permission(QueryKind::Environment)
    );
    assert_eq!(
        io_pending(QueryKind::Environment, &missing),
        CandidatePending::Query(QueryKind::Environment)
    );
}
