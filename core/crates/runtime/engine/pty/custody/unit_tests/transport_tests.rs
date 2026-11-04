use super::{CustodyDescriptorSet, CustodyUnixStream, tests::request};
use crate::protocol::RequestEnvelope;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

const PIPE_ENDPOINT_COUNT: usize = 2;
const SINGLE_DESCRIPTOR_COUNT: usize = 1;
const TRANSPORT_BYTE: u8 = b'P';

#[tokio::test]
async fn typed_transport_preserves_exact_descriptor_role() {
    let (left, right) = tokio::net::UnixStream::pair().unwrap();
    let sender = CustodyUnixStream::from_accepted(left).unwrap();
    let receiver = CustodyUnixStream::from_accepted(right).unwrap();
    let [reader, writer] = pipe();
    sender.send_rights(&[writer.as_fd()]).await.unwrap();
    let received = receiver
        .receive_rights(SINGLE_DESCRIPTOR_COUNT)
        .await
        .unwrap();
    assert_eq!(received.len(), SINGLE_DESCRIPTOR_COUNT);
    drop(writer);
    assert_eq!(
        unsafe {
            libc::write(
                received[0].as_raw_fd(),
                &TRANSPORT_BYTE as *const u8 as _,
                SINGLE_DESCRIPTOR_COUNT,
            )
        },
        SINGLE_DESCRIPTOR_COUNT as isize
    );
    let mut observed = u8::default();
    assert_eq!(
        unsafe {
            libc::read(
                reader.as_raw_fd(),
                &mut observed as *mut u8 as _,
                SINGLE_DESCRIPTOR_COUNT,
            )
        },
        SINGLE_DESCRIPTOR_COUNT as isize
    );
    assert_eq!(observed, TRANSPORT_BYTE);
}

#[tokio::test]
async fn released_successor_has_an_explicit_empty_rights_phase() {
    let (left, right) = tokio::net::UnixStream::pair().unwrap();
    let sender = CustodyUnixStream::from_accepted(left).unwrap();
    let receiver = CustodyUnixStream::from_accepted(right).unwrap();

    sender.send_rights(&[]).await.unwrap();
    let received = receiver
        .receive_rights(CustodyDescriptorSet::ReleasedSuccessor.descriptor_count())
        .await
        .unwrap();

    assert!(received.is_empty());
}

#[tokio::test]
async fn opening_uses_the_reserved_method_and_typed_request() {
    let (left, right) = tokio::net::UnixStream::pair().unwrap();
    let mut sender = CustodyUnixStream::from_accepted(left).unwrap();
    let mut receiver = CustodyUnixStream::from_accepted(right).unwrap();
    let expected = request();

    sender.write_opening(&expected).await.unwrap();
    let opening: RequestEnvelope = receiver.read().await.unwrap();
    let observed = CustodyUnixStream::decode_opening(&opening).unwrap();

    assert_eq!(observed, expected);
}

fn pipe() -> [OwnedFd; PIPE_ENDPOINT_COUNT] {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        libc::c_int::default()
    );
    descriptors.map(|descriptor| unsafe { OwnedFd::from_raw_fd(descriptor) })
}
