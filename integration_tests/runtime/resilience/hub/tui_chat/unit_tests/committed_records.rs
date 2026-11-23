use crate::submissions::committed;

const RECORD_END: u8 = b'\n';
const MESSAGES: [&str; 3] = ["中文🙂", "a \"quote\" and \\ escape\nnext line", ""];

#[test]
fn every_append_boundary_preserves_only_complete_records() {
    let mut bytes = Vec::new();
    let mut ends = Vec::new();
    for message in MESSAGES {
        serde_json::to_writer(&mut bytes, message).unwrap();
        bytes.push(RECORD_END);
        ends.push(bytes.len());
    }
    for boundary in 0..=bytes.len() {
        let complete = ends.iter().filter(|end| **end <= boundary).count();
        assert_eq!(
            committed(&bytes[..boundary]).unwrap(),
            MESSAGES[..complete],
            "append boundary {boundary}"
        );
    }
}

#[test]
fn valid_json_without_commit_delimiter_is_still_pending() {
    let bytes = serde_json::to_vec(MESSAGES[0]).unwrap();
    assert!(committed(&bytes).unwrap().is_empty());
}

#[test]
fn every_complete_corrupt_record_is_rejected_even_before_an_unfinished_tail() {
    const CORRUPT: [&[u8]; 6] = [
        b"\"\xff\"\n",
        b"not-json\n",
        b"\"unterminated\n",
        b"\n",
        b"{}\n",
        b"\"ok\" trailing\n",
    ];
    const COMPLETE: &[u8] = b"\"prior\"\n";
    const UNFINISHED: &[u8] = b"\"\xe4";
    for corrupt in CORRUPT {
        assert!(committed(corrupt).is_err());
        let bytes = [COMPLETE, corrupt, UNFINISHED].concat();
        assert!(committed(&bytes).is_err());
    }
}
