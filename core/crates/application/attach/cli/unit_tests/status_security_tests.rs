use super::*;
#[cfg(target_os = "macos")]
mod ancillary {
    use super::*;
    use crate::status::ancillary_test_support::{
        ANCILLARY_MARKER, assert_reader_eof, pipe, send_right,
    };
    use std::io::ErrorKind;
    use std::os::fd::AsRawFd;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use yolo_runtime::local_unix::LocalUnixStream;

    #[tokio::test]
    async fn socket_reporter_rejects_and_closes_an_unexpected_right() {
        let (sender, stream) = LocalUnixStream::pair().unwrap();
        let transferred = pipe();
        send_right(sender.as_raw_fd(), transferred.writer.as_raw_fd());
        drop(transferred.writer);
        let (_event_sender, events) = mpsc::unbounded_channel();

        let error = StatusReporter::Socket(stream)
            .run(events)
            .await
            .unwrap_err();

        assert_anyhow_io_kind(&error, ErrorKind::InvalidData);
        assert_reader_eof(&transferred.reader);
    }

    #[tokio::test]
    async fn ancillary_failure_poisons_both_halves_even_after_peer_eof() {
        let (sender, stream) = LocalUnixStream::pair().unwrap();
        let transferred = pipe();
        send_right(sender.as_raw_fd(), transferred.writer.as_raw_fd());
        drop(transferred.writer);
        let (mut reader, mut writer) = tokio::io::split(stream);
        let mut marker = u8::default();

        let read_error = reader
            .read(std::slice::from_mut(&mut marker))
            .await
            .unwrap_err();
        assert_eq!(read_error.kind(), ErrorKind::InvalidData);
        assert_reader_eof(&transferred.reader);

        let write_error = writer.write_all(&[ANCILLARY_MARKER]).await.unwrap_err();
        assert_eq!(write_error.kind(), ErrorKind::InvalidData);

        drop(sender);
        let eof_error = reader
            .read(std::slice::from_mut(&mut marker))
            .await
            .unwrap_err();
        assert_eq!(eof_error.kind(), ErrorKind::InvalidData);
    }

    fn assert_anyhow_io_kind(error: &anyhow::Error, expected: ErrorKind) {
        assert_eq!(
            error.downcast_ref::<std::io::Error>().map(|io| io.kind()),
            Some(expected)
        );
    }
}
