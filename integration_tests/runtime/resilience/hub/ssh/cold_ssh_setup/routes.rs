use super::*;

pub async fn wait_hub(entry: &RelayProcess, hub: HubId) {
    let end = deadline();
    tokio::time::timeout_at(end, async {
        loop {
            if matches!(
                entry
                    .client()
                    .await
                    .call(
                        hub,
                        Command::List {
                            include_archived: false,
                            after: None
                        },
                        end
                    )
                    .await,
                Ok(Reply::Terminals { .. })
            ) {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn wait_route(entry: &RelayProcess, target: RuntimeTarget) {
    let end = deadline();
    tokio::time::timeout_at(end, async {
        loop {
            if let Ok(Reply::Terminal { terminal: session }) = entry
                .client()
                .await
                .call(
                    target.address.hub_id,
                    Command::Get {
                        address: target.address,
                    },
                    end,
                )
                .await
            {
                assert_eq!(session.fields().runtime.unwrap().target, target);
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}
