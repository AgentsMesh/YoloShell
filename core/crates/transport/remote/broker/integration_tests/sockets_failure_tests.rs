use super::*;
use std::ffi::OsStr;
use std::mem::ManuallyDrop;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;

const NUL_SOCKET_NAME: &[u8] = b"invalid\0.sock";
const ROOT_PATH: &str = "/";
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;

#[test]
fn endpoint_name_rejects_root_and_nul_file_names() {
    assert!(endpoint_name(Path::new(ROOT_PATH)).is_err());
    let nul_path = Path::new(OsStr::from_bytes(NUL_SOCKET_NAME));
    assert!(endpoint_name(nul_path).is_err());
}

#[tokio::test]
async fn peer_verification_reports_a_closed_stream_descriptor() {
    let (stream, _peer) = UnixStream::pair().unwrap();
    let stream = ManuallyDrop::new(stream);
    let descriptor = (&*stream).as_raw_fd();
    assert_eq!(unsafe { libc::close(descriptor) }, SYSTEM_CALL_SUCCESS);

    assert!(verify_peer(&*stream).is_err());
}
