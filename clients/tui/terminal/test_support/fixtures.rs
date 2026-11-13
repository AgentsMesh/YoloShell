use yolo_hub_identity::*;
use yolo_hub_terminal_protocol::*;
pub const CONTENT: &[u8] = b"hello";
pub const NEXT_CONTENT: &[u8] = b" world";
pub const SNAPSHOT_SEQUENCE: u64 = 4;
pub const ROWS: u16 = 24;
pub const COLUMNS: u16 = 80;

pub fn snapshot() -> TerminalSnapshot {
    TerminalSnapshot::new(TerminalSnapshotFields {
        target: RuntimeTarget {
            address: TerminalAddress {
                hub_id: HubId::new(),
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        barrier_sequence: SNAPSHOT_SEQUENCE,
        size: TerminalSize::new(COLUMNS, ROWS).unwrap(),
        data: CONTENT.to_vec(),
    })
    .unwrap()
}

pub fn append(snapshot: &TerminalSnapshot) -> AttachResult {
    let fields = snapshot.fields();
    let sequence = fields.barrier_sequence + FIRST_OUTPUT_SEQUENCE;
    AttachResult::new(AttachResultFields {
        target: fields.target,
        snapshot: None,
        chunks: vec![
            OutputChunk::new(OutputChunkFields {
                target: fields.target,
                sequence,
                data: NEXT_CONTENT.to_vec(),
            })
            .unwrap(),
        ],
        history: None,
        next_sequence: sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: false,
    })
    .unwrap()
}
