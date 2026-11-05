use crate::{
    constants::SHUTDOWN_TIMEOUT, harness::Harness, operations, peer::*, queries, query_requests::*,
};

#[tokio::test]
async fn closing_or_dropping_the_owner_retires_opening_and_dispatched_query_sockets() {
    for cancel in [false, true] {
        let mut harness = Harness::start();
        let mut control = harness.accept().await;
        let generation = harness.ready().await;
        let (mut dispatched, _) = queries::dispatched(&harness, generation).await;
        harness.send(Some(generation), query()).await;
        let (mut opening, _) = operations::awaiting_handshake(&harness).await;
        if cancel {
            harness.cancel().await;
        } else {
            harness.finish().await;
        }
        tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
            tokio::join!(eof(&mut control), eof(&mut dispatched), eof(&mut opening));
        })
        .await
        .unwrap();
    }
}
