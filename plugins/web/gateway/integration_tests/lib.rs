mod admission;
mod backpressure;
mod blocked_input;
#[path = "../src/bridge.rs"]
mod bridge;
pub(crate) use yolo_web_gateway::{INPUT_QUEUE_CAPACITY, PING_QUEUE_CAPACITY, STREAM_CHUNK_BYTES};
mod protocol_edges;
mod streams;
#[path = "../test_support/mod.rs"]
mod support;
