use crate::claude_hooks::{self, ClaudeInteractionDecision, ClaudeLifecycleEvent};
use crate::delivery_test_support::*;
use crate::permission;

#[tokio::test]
async fn permission_request_returns_the_exact_allow_decision() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let server = tokio::spawn(async move {
        let (mut stream, request) = receive(&listener).await;
        let ClaudeLifecycleEvent::InteractionRequested {
            interaction,
            tool_name,
            prompt,
            ..
        } = &request.event
        else {
            panic!("expected an interaction request");
        };
        assert_eq!(tool_name, "Bash");
        assert!(prompt.contains("cargo test"));
        let mut response = accepted(&request);
        response.interaction = Some(ClaudeInteractionDecision {
            interaction_id: *interaction,
            allow: true,
        });
        respond(&mut stream, &response).await;
    });
    let decision = tokio::time::timeout(
        DELIVERY_BUDGET,
        permission::request_decision(
            &fixture.client,
            fixture.session.clone(),
            "Bash",
            Some(&serde_json::json!({ "command": "cargo test" })),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    assert_eq!(decision, Some(true));
}

#[tokio::test]
async fn a_not_ready_response_is_retried_until_the_binding_is_ready() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let server = tokio::spawn(async move {
        let (mut stream, request) = receive(&listener).await;
        let mut not_ready = accepted(&request);
        not_ready.accepted = false;
        not_ready.code = claude_hooks::CLAUDE_HOOK_NOT_READY.to_owned();
        respond(&mut stream, &not_ready).await;

        let (mut stream, request) = receive(&listener).await;
        let ClaudeLifecycleEvent::InteractionRequested { interaction, .. } = &request.event else {
            panic!("expected an interaction request");
        };
        let mut response = accepted(&request);
        response.interaction = Some(ClaudeInteractionDecision {
            interaction_id: *interaction,
            allow: true,
        });
        respond(&mut stream, &response).await;
    });
    let decision = tokio::time::timeout(
        DELIVERY_BUDGET,
        permission::request_decision(&fixture.client, fixture.session.clone(), "Bash", None),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    assert_eq!(decision, Some(true));
}

#[tokio::test]
async fn missing_or_mismatched_interaction_is_not_a_decision() {
    for mismatched in [false, true] {
        let fixture = Fixture::new();
        let listener = fixture.listener();
        let server = tokio::spawn(async move {
            let (mut stream, request) = receive(&listener).await;
            let mut response = accepted(&request);
            if mismatched {
                response.interaction = Some(ClaudeInteractionDecision {
                    interaction_id: uuid::Uuid::new_v4(),
                    allow: true,
                });
            }
            respond(&mut stream, &response).await;
        });
        let decision = tokio::time::timeout(
            DELIVERY_BUDGET,
            permission::request_decision(&fixture.client, fixture.session.clone(), "Bash", None),
        )
        .await
        .unwrap()
        .unwrap();
        server.await.unwrap();
        assert_eq!(decision, None);
    }
}

#[tokio::test]
async fn interaction_prompt_is_bounded_and_keeps_the_tool_name() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let server = tokio::spawn(async move {
        let (mut stream, request) = receive(&listener).await;
        let ClaudeLifecycleEvent::InteractionRequested {
            interaction,
            prompt,
            ..
        } = &request.event
        else {
            panic!("expected an interaction request");
        };
        assert!(prompt.starts_with("Bash: "));
        assert!(prompt.len() <= claude_hooks::MAX_CLAUDE_PERMISSION_ENVELOPE_BYTES);
        let mut response = accepted(&request);
        response.interaction = Some(ClaudeInteractionDecision {
            interaction_id: *interaction,
            allow: false,
        });
        respond(&mut stream, &response).await;
    });
    let huge = "x".repeat(claude_hooks::MAX_CLAUDE_PERMISSION_ENVELOPE_BYTES);
    let decision = tokio::time::timeout(
        DELIVERY_BUDGET,
        permission::request_decision(
            &fixture.client,
            fixture.session.clone(),
            "Bash",
            Some(&serde_json::json!({ "command": huge })),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    assert_eq!(decision, Some(false));
}
