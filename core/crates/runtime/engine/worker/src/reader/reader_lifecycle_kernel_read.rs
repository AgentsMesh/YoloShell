use super::quiescence::OutputReadAttempt;
use std::io::{self, Read};

#[cfg(test)]
const EXPECTED_KERNEL_EOF_DETAIL: &str = "expected a sealed PTY kernel EOF observation";

pub(crate) struct KernelPtyEofRead<'a> {
    attempt: OutputReadAttempt<'a>,
}

pub(crate) enum PtyReadOutcome<'a> {
    Bytes {
        attempt: OutputReadAttempt<'a>,
        length: usize,
    },
    Eof(KernelPtyEofRead<'a>),
    Failed {
        attempt: OutputReadAttempt<'a>,
        error: io::Error,
    },
}

#[cfg(test)]
impl<'a> PtyReadOutcome<'a> {
    pub(crate) fn expect_kernel_eof(self) -> KernelPtyEofRead<'a> {
        match self {
            Self::Eof(eof) => eof,
            _ => panic!("{EXPECTED_KERNEL_EOF_DETAIL}"),
        }
    }
}

impl<'a> OutputReadAttempt<'a> {
    pub(crate) fn read_from<Reader: Read + ?Sized>(
        self,
        reader: &mut Reader,
        buffer: &mut [u8],
    ) -> PtyReadOutcome<'a> {
        match reader.read(buffer) {
            Ok(0) => PtyReadOutcome::Eof(KernelPtyEofRead { attempt: self }),
            Ok(length) => PtyReadOutcome::Bytes {
                attempt: self,
                length,
            },
            Err(error) if crate::worker::pty_output_terminal::is_kernel_pty_eof_error(&error) => {
                PtyReadOutcome::Eof(KernelPtyEofRead { attempt: self })
            }
            Err(error) => PtyReadOutcome::Failed {
                attempt: self,
                error,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn kernel_eof_for_test(self) -> KernelPtyEofRead<'a> {
        KernelPtyEofRead { attempt: self }
    }
}

impl<'a> KernelPtyEofRead<'a> {
    pub(crate) fn into_attempt(self) -> OutputReadAttempt<'a> {
        self.attempt
    }
}
