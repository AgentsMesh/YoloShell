use super::*;
use serde::Deserialize;
use std::cell::Cell;
use std::io::Read;
use std::rc::Rc;
#[path = "../test_support/streaming_constants.rs"]
mod constants;
use constants::PLATFORM_TAG;

const HANDLE_ARRAY_START: &str = "\"bytes\":[";
const REPEATED_ELEMENT: &[u8] = b"1,";
const OVERFLOW_SENTINEL: usize = 1;
// serde_json::end_seq peeks once beyond the delimiter even after visitor Err.
const JSON_ERROR_LOOKAHEAD_BYTES: usize = 1;
const READER_EXHAUSTED: &str = "identity parser consumed beyond bounded handle";
const HANDLE_REJECTED: &str = "Linux handle exceeds capacity";

struct InfiniteHandle {
    prefix: Vec<u8>,
    consumed: usize,
    exhausted: Rc<Cell<bool>>,
}

impl Read for InfiniteHandle {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(usize::default());
        }
        let limit = self.prefix.len()
            + (HANDLE_CAPACITY + OVERFLOW_SENTINEL) * REPEATED_ELEMENT.len()
            + JSON_ERROR_LOOKAHEAD_BYTES;
        if self.consumed >= limit {
            self.exhausted.set(true);
            return Err(std::io::Error::other(READER_EXHAUSTED));
        }
        buffer[0] = self.prefix.get(self.consumed).copied().unwrap_or_else(|| {
            REPEATED_ELEMENT[(self.consumed - self.prefix.len()) % REPEATED_ELEMENT.len()]
        });
        self.consumed += 1;
        Ok(1)
    }
}

#[test]
fn entire_identity_rejects_unbounded_handle_before_tag_or_record_buffering() {
    let encoded = serde_json::to_string(&fixture::linux()).unwrap();
    let (reader, exhausted) = stream(&encoded);
    let error =
        DurableObjectIdentity::deserialize(&mut serde_json::Deserializer::from_reader(reader))
            .unwrap_err();

    assert!(error.is_data());
    assert!(error.to_string().contains(HANDLE_REJECTED));
    assert!(!exhausted.get(), "{READER_EXHAUSTED}");
}

fn stream(encoded: &str) -> (InfiniteHandle, Rc<Cell<bool>>) {
    let prefix_end = encoded.find(HANDLE_ARRAY_START).unwrap() + HANDLE_ARRAY_START.len();
    let exhausted = Rc::new(Cell::new(false));
    let reader = InfiniteHandle {
        prefix: encoded.as_bytes()[..prefix_end].to_vec(),
        consumed: usize::default(),
        exhausted: Rc::clone(&exhausted),
    };
    (reader, exhausted)
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct BufferedIdentity {
    proof: BufferedProof,
}

#[derive(Deserialize)]
#[allow(dead_code)]
#[serde(tag = "platform", rename_all = "snake_case")]
enum BufferedProof {
    Linux { handle: Handle },
}

#[test]
fn internally_tagged_negative_control_exhausts_the_same_reader_budget() {
    // This deliberately unsafe test-only parser demonstrates the regression
    // gate rejects buffering before the bounded Handle visitor is reached.
    let mut wire = serde_json::to_value(fixture::linux()).unwrap();
    let mut proof = wire["proof"]["linux"].take();
    proof
        .as_object_mut()
        .unwrap()
        .insert(PLATFORM_TAG.into(), serde_json::json!("linux"));
    wire["proof"] = proof;
    let (reader, exhausted) = stream(&serde_json::to_string(&wire).unwrap());
    let result = BufferedIdentity::deserialize(&mut serde_json::Deserializer::from_reader(reader));

    assert!(result.err().unwrap().is_io());
    assert!(exhausted.get());
}
