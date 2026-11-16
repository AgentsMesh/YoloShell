use std::cell::RefCell;

const EXPECTED_PID: u32 = 42;
const EXPECTED_START_TIME: u64 = 7;
const REPLACEMENT_START_TIME: u64 = 8;
const TEST_DESCRIPTOR: u8 = 11;
const TEST_FAILURE_DETAIL: &str = "injected pidfd failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Call {
    Open(u32),
    StartTime(u32),
    Exited,
    Send(i32),
}

#[derive(Clone, Copy)]
enum IoOutcome {
    Success,
    OsError(i32),
}

#[derive(Clone, Copy)]
enum StartTimeOutcome {
    Value(Option<u64>),
    Failure,
}

struct FakeOperations {
    calls: RefCell<Vec<Call>>,
    open: IoOutcome,
    start_time: StartTimeOutcome,
    exited: io::Result<bool>,
    send: IoOutcome,
}

impl FakeOperations {
    fn successful() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            open: IoOutcome::Success,
            start_time: StartTimeOutcome::Value(Some(EXPECTED_START_TIME)),
            exited: Ok(false),
            send: IoOutcome::Success,
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }
}

impl PidfdOperations for FakeOperations {
    type Descriptor = u8;

    fn open(&self, pid: u32) -> io::Result<Self::Descriptor> {
        self.calls.borrow_mut().push(Call::Open(pid));
        io_outcome(self.open).map(|()| TEST_DESCRIPTOR)
    }

    fn start_time(&self, pid: u32) -> Result<Option<u64>> {
        self.calls.borrow_mut().push(Call::StartTime(pid));
        match self.start_time {
            StartTimeOutcome::Value(value) => Ok(value),
            StartTimeOutcome::Failure => anyhow::bail!(TEST_FAILURE_DETAIL),
        }
    }

    fn exited(&self, _descriptor: &Self::Descriptor) -> io::Result<bool> {
        self.calls.borrow_mut().push(Call::Exited);
        self.exited
            .as_ref()
            .copied()
            .map_err(|error| io::Error::from_raw_os_error(error.raw_os_error().unwrap()))
    }

    fn send(&self, _descriptor: &Self::Descriptor, signal: i32) -> io::Result<()> {
        self.calls.borrow_mut().push(Call::Send(signal));
        io_outcome(self.send)
    }
}

fn identity() -> ProcessIdentity {
    ProcessIdentity {
        pid: EXPECTED_PID,
        start_time: EXPECTED_START_TIME,
    }
}

fn io_outcome(outcome: IoOutcome) -> io::Result<()> {
    match outcome {
        IoOutcome::Success => Ok(()),
        IoOutcome::OsError(code) => Err(io::Error::from_raw_os_error(code)),
    }
}
