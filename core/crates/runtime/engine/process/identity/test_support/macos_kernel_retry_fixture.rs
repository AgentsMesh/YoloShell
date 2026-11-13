use super::super::super::{
    EXECUTABLE_UUID_BYTES, MACOS_PROC_PIDINFO_NO_ARGUMENT, PROC_PIDT_BSDINFOWITHUNIQID,
    ProcBsdInfoWithUniqueId, ProcUniqueIdentifierInfo,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    mem::size_of,
};

#[path = "macos_kernel_retry_constants.rs"]
pub mod constants;
use constants::*;

#[derive(Clone, Copy)]
pub enum Exact {
    Error(i32),
    Active,
    Changed,
    Executed,
    WrongPid,
    MissingUnique,
    Short,
    NoErrno,
    Zombie,
}

#[derive(Clone, Copy)]
pub enum Basic {
    Active,
    Denied,
    Gone,
}

thread_local! {
    static RESPONSES: RefCell<VecDeque<Exact>> = const { RefCell::new(VecDeque::new()) };
    static BASIC: Cell<Basic> = const { Cell::new(Basic::Active) };
    static EXACT_CALLS: Cell<usize> = const { Cell::new(0) };
    static BASIC_CALLS: Cell<usize> = const { Cell::new(0) };
}

pub fn setup(exact: impl IntoIterator<Item = Exact>, basic: Basic) {
    RESPONSES.with(|responses| *responses.borrow_mut() = exact.into_iter().collect());
    BASIC.set(basic);
    EXACT_CALLS.set(0);
    BASIC_CALLS.set(0);
}

pub fn counts() -> (usize, usize) {
    (EXACT_CALLS.get(), BASIC_CALLS.get())
}

pub unsafe extern "C" fn query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    if pid != TEST_PID as i32 || argument != MACOS_PROC_PIDINFO_NO_ARGUMENT {
        return fail(libc::EINVAL);
    }
    if flavor == PROC_PIDT_BSDINFOWITHUNIQID {
        EXACT_CALLS.set(EXACT_CALLS.get() + QUERY_INCREMENT);
        let response = RESPONSES.with(|responses| responses.borrow_mut().pop_front());
        return match response {
            Some(Exact::Error(error)) => fail(error),
            Some(Exact::Short) => SHORT_RESPONSE,
            Some(Exact::NoErrno) => 0,
            Some(response) => write(buffer, buffer_size, raw(response)),
            None => fail(libc::EIO),
        };
    }
    if flavor == libc::PROC_PIDTBSDINFO {
        BASIC_CALLS.set(BASIC_CALLS.get() + QUERY_INCREMENT);
        return match BASIC.get() {
            Basic::Active => write(buffer, buffer_size, raw(Exact::Active).bsd),
            Basic::Denied => fail(libc::EPERM),
            Basic::Gone => fail(libc::ESRCH),
        };
    }
    fail(libc::EINVAL)
}

fn raw(response: Exact) -> ProcBsdInfoWithUniqueId {
    let mut bsd = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    bsd.pbi_pid = if matches!(response, Exact::WrongPid) {
        WRONG_PID
    } else {
        TEST_PID
    };
    bsd.pbi_status = if matches!(response, Exact::Zombie) {
        libc::SZOMB
    } else {
        libc::SRUN
    };
    bsd.pbi_ppid = PARENT_PID;
    bsd.pbi_pgid = PROCESS_GROUP;
    bsd.e_tpgid = FOREGROUND_GROUP;
    bsd.pbi_start_tvsec = START_SECONDS;
    bsd.pbi_start_tvusec = if matches!(response, Exact::Changed) {
        CHANGED_START_MICROSECONDS
    } else {
        START_MICROSECONDS
    };
    ProcBsdInfoWithUniqueId {
        bsd,
        unique: ProcUniqueIdentifierInfo {
            executable_uuid: [0; EXECUTABLE_UUID_BYTES],
            unique_id: match response {
                Exact::MissingUnique => 0,
                Exact::Changed => CHANGED_UNIQUE_ID,
                _ => UNIQUE_ID,
            },
            parent_unique_id: PARENT_UNIQUE_ID,
            pid_version: if matches!(response, Exact::Changed | Exact::Executed) {
                CHANGED_PID_VERSION
            } else {
                PID_VERSION
            },
            original_parent_pid_version: 0,
            reserved_2: 0,
            reserved_3: 0,
        },
    }
}

fn write<T>(buffer: *mut libc::c_void, buffer_size: i32, value: T) -> i32 {
    let expected = size_of::<T>() as i32;
    if buffer.is_null() || buffer_size != expected {
        return fail(libc::EINVAL);
    }
    unsafe { buffer.cast::<T>().write(value) };
    expected
}

fn fail(error: i32) -> i32 {
    unsafe { *libc::__error() = error };
    QUERY_FAILURE
}
