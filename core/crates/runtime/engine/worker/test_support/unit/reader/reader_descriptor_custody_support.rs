#[derive(Clone, Copy)]
struct DescriptorCloseObservation {
    descriptor: RawFd,
    result: libc::c_int,
    error: Option<i32>,
}

struct ObservedDescriptorReader {
    socket: Option<UnixStream>,
    dropping: mpsc::SyncSender<RawFd>,
    closed: mpsc::SyncSender<DescriptorCloseObservation>,
    release: Option<mpsc::Receiver<()>>,
}

impl Read for ObservedDescriptorReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.socket.as_mut().unwrap().read(buffer)
    }
}

impl Drop for ObservedDescriptorReader {
    fn drop(&mut self) {
        let descriptor = self.socket.as_ref().unwrap().as_raw_fd();
        let _ = self.dropping.send(descriptor);
        if let Some(release) = self.release.take() {
            let _ = release.recv();
        }
        let raw_descriptor = self.socket.take().unwrap().into_raw_fd();
        let result = unsafe { libc::close(raw_descriptor) };
        let error = (result == CUSTODY_CLOSE_FAILURE)
            .then(|| io::Error::last_os_error().raw_os_error())
            .flatten();
        let _ = self.closed.send(DescriptorCloseObservation {
            descriptor,
            result,
            error,
        });
    }
}

fn observed_reader(
    release: Option<mpsc::Receiver<()>>,
) -> (
    ObservedDescriptorReader,
    mpsc::Receiver<RawFd>,
    mpsc::Receiver<DescriptorCloseObservation>,
) {
    let (socket, peer) = UnixStream::pair().unwrap();
    drop(peer);
    let (dropping, drop_observed) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
    let (closed, close_observed) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
    (
        ObservedDescriptorReader {
            socket: Some(socket),
            dropping,
            closed,
            release,
        },
        drop_observed,
        close_observed,
    )
}

fn assert_descriptor_open(descriptor: RawFd) {
    assert_ne!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
        CUSTODY_FCNTL_FAILURE
    );
}

fn assert_descriptor_closed(observation: DescriptorCloseObservation) {
    assert_eq!(observation.result, CUSTODY_CLOSE_SUCCESS);
    assert_eq!(observation.error, None);
}
