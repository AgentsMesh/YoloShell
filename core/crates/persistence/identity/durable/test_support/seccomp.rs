use std::io;

const SYSCALL_NUMBER_OFFSET: u32 = 0;
const NO_JUMP: u8 = 0;
const SKIP_DENIAL: u8 = 1;
const ENABLE: libc::c_ulong = 1;
const UNUSED: libc::c_ulong = 0;

/// Apply only inside a fresh child executable. All descriptors remain valid;
/// the kernel itself rejects one selected syscall with the requested errno.
pub fn deny(syscall: libc::c_long, errno: libc::c_int) -> io::Result<()> {
    let mut instructions = [
        instruction(
            (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16,
            NO_JUMP,
            NO_JUMP,
            SYSCALL_NUMBER_OFFSET,
        ),
        instruction(
            (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16,
            NO_JUMP,
            SKIP_DENIAL,
            syscall as u32,
        ),
        instruction(
            (libc::BPF_RET | libc::BPF_K) as u16,
            NO_JUMP,
            NO_JUMP,
            libc::SECCOMP_RET_ERRNO | errno as u32,
        ),
        instruction(
            (libc::BPF_RET | libc::BPF_K) as u16,
            NO_JUMP,
            NO_JUMP,
            libc::SECCOMP_RET_ALLOW,
        ),
    ];
    let program = libc::sock_fprog {
        len: instructions.len() as u16,
        filter: instructions.as_mut_ptr(),
    };
    // SAFETY: no-new-privs affects this child only; all variadic words are present.
    let status = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, ENABLE, UNUSED, UNUSED, UNUSED) };
    if status != libc::c_int::default() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel copies the complete BPF program synchronously. Every
    // jump remains inside this array, with explicit allow as the final rule.
    let status = unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER as libc::c_ulong,
            std::ptr::from_ref(&program),
            UNUSED,
            UNUSED,
        )
    };
    if status == libc::c_int::default() {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn instruction(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}
