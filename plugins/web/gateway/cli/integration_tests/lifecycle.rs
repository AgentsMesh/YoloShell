use crate::{backend::Backend, browser, child::Child, configuration::Configuration, constants::*};
use tokio::{net::TcpListener, time::timeout_at};

#[tokio::test]
async fn actual_cli_loads_private_files_authenticates_hub_and_cleans_up_on_sigint() {
    let configuration = Configuration::new();
    let backend = Backend::start(&configuration).await;
    let mut child = Child::start(&configuration);
    let address = child.ready().await;
    let (mut browser, ready) = browser::login(address).await;
    assert_eq!(
        ready[HELLO_FIELD][HUB_FIELD],
        configuration.hub.hub_id.to_string()
    );
    assert_eq!(
        ready[HELLO_FIELD][METHODS_FIELD],
        serde_json::json!(["hello"])
    );
    child.stop().await;
    browser::expect_closed(&mut browser).await;
    timeout_at(deadline(), backend.closed.notified())
        .await
        .unwrap();
    let rebound = TcpListener::bind(address)
        .await
        .expect("CLI retained its listener after exit");
    drop(rebound);
}
