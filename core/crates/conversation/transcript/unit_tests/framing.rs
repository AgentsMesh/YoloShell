use super::*;
use std::io::{self, Read};

#[test]
fn split_utf8_and_partial_json_do_not_publish_until_newline() {
    let line = "{\"text\":\"你好\"}\n".as_bytes();
    let mut framer = JsonlFramer::default();
    let mut records = Vec::new();
    for chunk in line.chunks(1) {
        framer.push(chunk, |start, end, parsed| {
            records.push((start, end, parsed))
        });
    }
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, 0);
    assert_eq!(records[0].1, line.len() as u64);
    assert_eq!(records[0].2.as_ref().unwrap()["text"], "你好");
    assert!(!framer.incomplete());
    framer.push(b"{}", |_, _, _| panic!("partial line published"));
    assert!(framer.incomplete());
}

#[test]
fn malformed_and_oversized_lines_do_not_poison_next_record() {
    let mut framer = JsonlFramer::default();
    let mut records = Vec::new();
    let mut bytes = vec![b'x'; MAX_RECORD_BYTES + 1];
    bytes.extend_from_slice(b"\nnot-json\n  \n{\"text\":\"ok\"}\n");
    framer.push(&bytes, |_, _, parsed| records.push(parsed));
    assert_eq!(records.len(), 3);
    assert_eq!(records[0], Err(ConversationPartialReason::OversizedRecord));
    assert_eq!(records[1], Err(ConversationPartialReason::MalformedRecord));
    assert_eq!(records[2].as_ref().unwrap()["text"], "ok");
}

#[test]
fn scanning_hashes_the_old_prefix_and_reports_incomplete_tail() {
    let old = b"{\"text\":\"one\"}\n";
    let mut bytes = old.to_vec();
    bytes.extend_from_slice(b"{\"text\":\"two\"}\n{\"text\":");
    let result = scan(
        bytes.as_slice(),
        bytes.len() as u64,
        old.len() as u64,
        u64::MAX,
        ConversationLimits::default(),
        &TestParser,
    )
    .unwrap();
    assert_eq!(result.previous_prefix_digest, yolo_crypto::sha256_hex(old));
    assert_eq!(result.digest, yolo_crypto::sha256_hex(&bytes));
    assert_eq!(result.selection.entries.len(), 2);
    assert_eq!(
        result.selection.partial,
        vec![ConversationPartialReason::IncompleteRecord]
    );
}

struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("fixture"))
    }
}

#[test]
fn source_limits_short_reads_and_io_errors_fail_explicitly() {
    let run = |reader, length| {
        scan(
            reader,
            length,
            0,
            u64::MAX,
            ConversationLimits::default(),
            &TestParser,
        )
    };
    assert!(matches!(
        run(&b""[..], MAX_HISTORY_SCAN_BYTES + 1),
        Err(ConversationError::HistoryLimitExceeded)
    ));
    assert!(matches!(
        run(&b""[..], 1),
        Err(ConversationError::EvidenceChanged)
    ));
    assert!(matches!(
        scan(Broken, 1, 0, 1, ConversationLimits::default(), &TestParser),
        Err(ConversationError::EvidenceUnavailable)
    ));
    assert!(matches!(
        scan(
            &b""[..],
            0,
            0,
            0,
            ConversationLimits {
                entries: 0,
                bytes: 1
            },
            &TestParser
        ),
        Err(ConversationError::InvalidRequest)
    ));
}
