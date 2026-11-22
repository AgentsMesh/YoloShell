use super::{
    federation_agents::CreatedAgent,
    federation_support::{RunningNode, deadline},
};
use crate::Harness;
use std::collections::BTreeSet;
use yolo_hub_client::ClientError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, CreationQueryRequest, PublicError};

pub fn command(created: &CreatedAgent) -> Command {
    Command::QueryCreation {
        request: CreationQueryRequest::new(created.request.clone()),
    }
}

pub async fn verify(entries: &[&RunningNode], created: &CreatedAgent) {
    for entry in entries {
        let mut client = entry.client().await;
        let Reply::CreationQuery { query } = client
            .call(created.target.address.hub_id, command(created), deadline())
            .await
            .unwrap()
        else {
            panic!("creation query returned another operation's reply");
        };
        assert_eq!(
            query.creation_token(),
            created.request.fields().creation_token
        );
        assert_eq!(
            query.outcome(),
            &CreationQueryOutcome::Finalized {
                creation: created.creation
            },
        );
        assert_eq!(created.creation.address, created.target.address);
        assert!(client.is_usable());
    }
}

pub async fn relay_lost(
    entry: &RunningNode,
    origin: &RunningNode,
    hosts: &[&Harness],
    created: &CreatedAgent,
) {
    let before = session_identities(hosts).await;
    let mut client = entry.client().await;
    let error = client
        .call(created.target.address.hub_id, command(created), deadline())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Remote(PublicError::Unreachable)
    ));
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
    verify(&[origin], created).await;
    // A lost route cannot fall back to Create on the entry, relay or origin.
    assert_eq!(session_identities(hosts).await, before);
}

async fn session_identities(hosts: &[&Harness]) -> Vec<BTreeSet<uuid::Uuid>> {
    let mut result = Vec::new();
    for host in hosts {
        let terminals = host.client().list_sessions_complete(true).await.unwrap();
        result.push(
            terminals
                .into_iter()
                .map(|view| view.manifest.terminal_id)
                .collect(),
        );
    }
    result
}
