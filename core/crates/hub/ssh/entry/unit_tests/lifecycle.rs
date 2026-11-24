use super::*;
use crate::{lifecycle::Lifecycle, socket::SocketOwner};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    time::timeout,
};

#[tokio::test]
async fn closing_entry_wakes_current_and_future_connectors() {
    let lifecycle = std::sync::Arc::new(Lifecycle::default());
    let waiter = lifecycle.clone();
    let waiting = tokio::spawn(async move {
        waiter.closed().await;
    });
    tokio::task::yield_now().await;
    lifecycle.close();
    timeout(TIMEOUT, waiting).await.unwrap().unwrap();
    timeout(TIMEOUT, lifecycle.closed()).await.unwrap();
}

#[tokio::test]
async fn lifecycle_close_and_late_registration_both_shutdown_exact_owned_socket() {
    for close_first in [false, true] {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, u16::default()))
            .await
            .unwrap();
        let (left, right) = tokio::join!(
            TcpStream::connect(listener.local_addr().unwrap()),
            listener.accept()
        );
        let (_stream, owner) = SocketOwner::capture(left.unwrap()).unwrap();
        let mut right = right.unwrap().0;
        let lifecycle = Lifecycle::default();
        if close_first {
            lifecycle.close();
        }
        assert_eq!(lifecycle.retain(&owner.0), !close_first);
        lifecycle.close();
        assert!(owner.0.is_closed());
        assert!(timeout(TIMEOUT, right.read_u8()).await.unwrap().is_err());
    }
}
