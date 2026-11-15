use crate::HubEntryStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn remote_variant_delegates_bidirectional_io() {
    let (left, mut right) = tokio::io::duplex(64);
    let mut stream = HubEntryStream::Remote(Box::new(left));
    stream.write_all(b"ping").await.unwrap();
    let mut received = [0; 4];
    right.read_exact(&mut received).await.unwrap();
    assert_eq!(&received, b"ping");
    right.write_all(b"pong").await.unwrap();
    let mut reply = [0; 4];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"pong");
}
