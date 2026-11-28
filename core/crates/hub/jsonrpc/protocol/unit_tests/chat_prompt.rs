use serde_json::{Value, json};
use yolo_hub_jsonrpc_protocol::{CHAT_METHOD, Call, ClientCall, INVALID_PARAMS};
use yolo_hub_rpc_protocol::{Command, Method};
use yolo_hub_rpc_protocol_test_support::commands;

fn prompt() -> Value {
    let (command, _) = commands::samples()
        .into_iter()
        .find(|(c, _)| c.method() == Method::AgentPrompt)
        .unwrap();
    let mut value = serde_json::to_value(Call::from_request(commands::request(command))).unwrap();
    value["method"] = json!(CHAT_METHOD);
    value["params"]["operation"] = json!("prompt");
    value
}

#[test]
fn text_and_origin_resource_links_form_one_semantic_prompt() {
    let mut wire = prompt();
    let id = wire["params"]["request"][crate::constants::REQUEST_ID].clone();
    wire["params"]["request"]["prompt"] = json!([
        {"type":"text","text":"review "},{"type":"text","text":"both"},
        {"type":"resource_link","name":"文 件.png","uri":"file:///tmp/%E6%96%87%20%E4%BB%B6.png","mimeType":"image/png"},
        {"type":"resource_link","name":"notes","uri":"file:///tmp/notes"}
    ]);
    let call = ClientCall::parse(wire).unwrap();
    let Command::AgentPrompt { request } = &call.call.fields().command else {
        panic!()
    };
    assert_eq!(
        request.fields().prompt.text(),
        "review both\n[文 件.png](file:///tmp/%E6%96%87%20%E4%BB%B6.png)\n[notes](file:///tmp/notes)"
    );
    assert_eq!(json!(request.fields().request_id), id);
}

#[test]
fn unsupported_content_is_rejected_atomically_before_submission() {
    for block in [
        json!({"type":"image","data":"aGVsbG8=","mimeType":"image/png"}),
        json!({"type":"resource_link","name":"wrong origin","uri":"file://other/file"}),
        json!({"type":"resource_link","name":"newline","uri":"file:///tmp/a%0Ab"}),
        json!({"type":"resource_link","name":"line\nbreak","uri":"file:///tmp/file"}),
        json!({"type":"resource_link","name":"network","uri":"https://example.org/image"}),
    ] {
        let mut wire = prompt();
        wire["params"]["request"]["prompt"] = json!([{"type":"text","text":"read"}, block]);
        assert_eq!(
            ClientCall::parse(wire).unwrap_err().error.code,
            INVALID_PARAMS
        );
    }
}
