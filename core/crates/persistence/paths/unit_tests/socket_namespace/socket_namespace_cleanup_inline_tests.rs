use super::*;

#[test]
fn null_readdir_distinguishes_eof_from_failure() {
    assert!(finish_readdir(0).is_ok());
    assert!(finish_readdir(libc::EIO).is_err());
}
