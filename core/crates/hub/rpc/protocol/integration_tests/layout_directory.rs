use crate::fixtures::{constants::*, hub};
use uuid::Uuid;
use yolo_hub_jsonrpc_protocol::{Answer, Call, decode};
use yolo_hub_jsonrpc_transport::encode;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::{
    LayoutAddress, LayoutSummary, MAX_DOCUMENT_PANES, MAX_DOCUMENTS, MAX_TABS,
};

const MAX_DIRECTORY_WIRE_BYTES: usize = 128 * 1024;

#[test]
fn largest_directory_roundtrips_as_bounded_summaries_without_tab_payloads() {
    let owner = hub(HUB_ID);
    let mut documents: Vec<_> = (0..MAX_DOCUMENTS)
        .map(|_| LayoutSummary {
            address: LayoutAddress {
                hub_id: owner,
                layout_id: Uuid::new_v4(),
            },
            revision: u64::MAX,
            tab_count: MAX_TABS as u32,
            pane_count: MAX_DOCUMENT_PANES as u32,
        })
        .collect();
    documents.sort_by_key(|summary| summary.address);
    let request = Call::new(owner, Command::ListLayouts {}).unwrap();
    let response = Answer::new(&request, Reply::Layouts { documents }).unwrap();
    response.clone().reply(&request).unwrap();
    let frame = encode(&response).unwrap();
    assert!(frame.len() < MAX_DIRECTORY_WIRE_BYTES);
    let decoded: Answer = decode(&frame).unwrap();
    assert_eq!(decoded, response);
    decoded.reply(&request).unwrap();
}
