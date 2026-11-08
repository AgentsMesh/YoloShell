fn exercise_kernel_errors(locked_file: File, identity: ObjectIdentity) {
    let (socket, peer) = std::os::unix::net::UnixStream::pair().unwrap();
    assert_eq!(
        unsafe { libc::dup2(socket.as_raw_fd(), locked_file.as_raw_fd()) },
        locked_file.as_raw_fd()
    );
    drop(socket);
    drop(peer);
    assert!(ProcessFileLock::try_lock(&locked_file, identity, Mode::Exclusive).is_err());
    assert!(ProcessFileLock::lock_shared(&locked_file, identity).is_err());
    assert!(
        ProcessFileLock::try_acquire_object(identity, Mode::Exclusive)
            .unwrap()
            .is_some()
    );
}
