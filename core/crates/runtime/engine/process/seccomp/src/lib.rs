//! Reviewed owner for installing a syscall denylist in a running process.
//! The plugin runtime calls this after exec, when bubblewrap's own filter can
//! no longer deny `execve` without blocking the exec that creates the plugin.

const PR_SET_NO_NEW_PRIVS: libc::c_int = 38;
const PR_SET_SECCOMP: libc::c_int = 22;
const SECCOMP_MODE_FILTER: libc::c_uint = 2;
const AUDIT_ARCH_AARCH64: u32 = 0xC000_00B7;
const FILTER_ALLOW: u32 = 0x7FFF_0000;
const FILTER_KILL: u32 = 0x8000_0000;
const FILTER_ERRNO: u32 = 0x0005_0000;
const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JEQ: u16 = 0x10;
const BPF_RET: u16 = 0x06;
const BPF_K: u16 = 0x00;
const OFFSET_NR: u32 = 0;
const OFFSET_ARCH: u32 = 4;
const PREFIX_INSTRUCTIONS: usize = 4;
const INSTRUCTIONS_PER_SYSCALL: usize = 2;

#[repr(C)]
struct Filter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

pub fn install_filter(syscalls: &[u32]) -> Result<(), String> {
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        install_aarch64(syscalls)
    }
    #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
    {
        let _ = syscalls;
        Err("seccomp is unavailable on this platform".into())
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn install_aarch64(syscalls: &[u32]) -> Result<(), String> {
    if unsafe { libc::prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(format!(
            "PR_SET_NO_NEW_PRIVS failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut filter = program(syscalls);
    let len = u16::try_from(filter.len()).map_err(|_| "seccomp program is too large")?;
    let mut program = libc::sock_fprog {
        len,
        filter: filter.as_mut_ptr(),
    };
    let installed = unsafe { libc::prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &mut program) };
    if installed != 0 {
        return Err(format!(
            "PR_SET_SECCOMP failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn program(syscalls: &[u32]) -> Vec<libc::sock_filter> {
    let mut program =
        Vec::with_capacity(PREFIX_INSTRUCTIONS + syscalls.len() * INSTRUCTIONS_PER_SYSCALL);
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, OFFSET_ARCH));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_AARCH64, 1, 0));
    program.push(statement(BPF_RET | BPF_K, FILTER_KILL));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, OFFSET_NR));
    for syscall in syscalls {
        program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, *syscall, 0, 1));
        program.push(statement(
            BPF_RET | BPF_K,
            FILTER_ERRNO | libc::EPERM as u32,
        ));
    }
    program.push(statement(BPF_RET | BPF_K, FILTER_ALLOW));
    program
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn statement(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn jump(code: u16, k: u32, jt: u8, jf: u8) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}
