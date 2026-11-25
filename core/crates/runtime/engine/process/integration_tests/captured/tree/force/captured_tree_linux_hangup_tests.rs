use super::{drained_hangup, ioctl, with_sealed_hangup};
use crate::process::PtySessionProof;
use portable_pty::{PtyPair, PtySize, native_pty_system};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::BorrowedFd;
use std::os::unix::fs::OpenOptionsExt;

fn pair() -> PtyPair {
    native_pty_system().openpty(PtySize::default()).unwrap()
}

fn master(pair: &PtyPair) -> File {
    let fd = pair.master.as_raw_fd().unwrap();
    // The portable PTY owns the descriptor for the duration of this borrow.
    File::from(
        unsafe { BorrowedFd::borrow_raw(fd) }
            .try_clone_to_owned()
            .unwrap(),
    )
}

fn slave(pair: &PtyPair) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(pair.master.tty_name().unwrap())
        .unwrap()
}

fn locked(master: &File) -> bool {
    let mut lock = 0;
    ioctl(master, libc::TIOCGPTLCK, &mut lock).unwrap();
    lock != 0
}

#[test]
fn held_slave_and_buffered_tail_prevent_retirement() {
    let pair = pair();
    let mut master = master(&pair);
    let mut slave = slave(&pair);
    drop(pair);
    assert!(!with_sealed_hangup(&master, || panic!("live holder")).unwrap());
    assert!(!locked(&master));
    slave.write_all(b"tail").unwrap();
    drop(slave);
    assert!(!with_sealed_hangup(&master, || panic!("undrained bytes")).unwrap());
    let mut data = [0; 4];
    master.read_exact(&mut data).unwrap();
    assert_eq!(&data, b"tail");
    assert!(with_sealed_hangup(&master, || Ok(true)).unwrap());
    assert!(locked(&master));
}

#[test]
fn failed_or_racing_cgroup_settlement_restores_the_original_lock() {
    let pair = pair();
    let master = master(&pair);
    drop(pair);
    assert!(!with_sealed_hangup(&master, || Ok(false)).unwrap());
    assert!(!locked(&master));
    assert!(with_sealed_hangup(&master, || anyhow::bail!("sample failed")).is_err());
    assert!(!locked(&master));
    ioctl(&master, libc::TIOCSPTLCK, &mut 1).unwrap();
    assert!(!with_sealed_hangup(&master, || Ok(false)).unwrap());
    assert!(locked(&master));
}

#[test]
fn proof_rejects_another_hung_up_master_and_accepts_its_own_alias_after_seal() {
    let own = pair();
    let other = pair();
    let proof = PtySessionProof::capture_bound(own.master.as_ref(), "owned-pty".into()).unwrap();
    let own_master = master(&own);
    let other_master = master(&other);
    drop((own, other));
    assert!(drained_hangup(&other_master).unwrap());
    assert!(!proof.matches_linux_master(&other_master).unwrap());
    assert!(with_sealed_hangup(&own_master, || Ok(true)).unwrap());
    assert!(
        proof
            .matches_linux_master(&own_master.try_clone().unwrap())
            .unwrap()
    );
    assert!(drained_hangup(&own_master).unwrap());
    assert!(locked(&own_master));
}
