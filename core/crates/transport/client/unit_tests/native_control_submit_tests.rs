use super::*;

const PROMPT: &str = "Use the existing agent conversation";

fn submit_request() -> NativeSubmitRequest {
    let interrupt = interrupt_request();
    NativeSubmitRequest {
        terminal_id: interrupt.terminal_id,
        runtime_epoch: interrupt.runtime_epoch,
        request_id: interrupt.request_id,
        generation: interrupt.generation,
        writer_capability: interrupt.writer_capability,
        witness: interrupt.witness,
        command_digest: interrupt.command_digest,
        prompt: yolo_agent_input::Prompt::new(PROMPT.into()).unwrap(),
    }
}

#[tokio::test]
async fn native_prompt_transmits_exact_authority_and_rejects_uncorrelated_delivery() {
    let request = submit_request();
    let expected = NativeControlResult {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
        outcome: NativeControlOutcome::Delivered,
    };
    let server = ReplyServer::start(method::SUBMIT_NATIVE_AGENT, &expected).await;
    assert_eq!(
        server.client.submit_native_agent(&request).await.unwrap(),
        expected
    );
    let sent: NativeSubmitRequest = server.finish().await.decode_payload().unwrap();
    assert_eq!(
        serde_json::to_value(sent).unwrap(),
        serde_json::to_value(&request).unwrap()
    );
    for mutate in RESULT_MUTATIONS {
        let mut reply = expected.clone();
        mutate(&mut reply);
        let server = ReplyServer::start(method::SUBMIT_NATIVE_AGENT, &reply).await;
        assert_mismatch(server.client.submit_native_agent(&request).await);
        server.finish().await;
    }
    let server = ReplyServer::failure(
        method::SUBMIT_NATIVE_AGENT,
        crate::protocol::ErrorBody::conflict(REMOTE_DENIAL),
    )
    .await;
    assert!(matches!(
        server.client.submit_native_agent(&request).await,
        Err(ClientError::Remote { .. })
    ));
    server.finish().await;
}
