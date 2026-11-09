use crate::host::tests::*;
const SERVER_WAVE5_SOCKET_NAME: &str = "wave5-ingress.sock";

#[tokio::test]
async fn spawned_ingress_arms_and_shuts_down_without_a_client() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let listener = UnixListener::bind(directory.path().join(SERVER_WAVE5_SOCKET_NAME)).unwrap();
    let (armed, ready) = tokio::sync::oneshot::channel();
    let mut ingress = HostIngress::spawn(host, listener, armed);

    ready.await.unwrap();
    ingress.shutdown().await.unwrap();
}
