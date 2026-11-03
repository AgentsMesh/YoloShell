use super::*;

const FIRST_NON_PIPE_DESCRIPTOR: i32 = 11;
const PIPE_DESCRIPTOR: i32 = 12;
const SECOND_NON_PIPE_DESCRIPTOR: i32 = 13;
const EXPECTED_PIPE_DESCRIPTOR_COUNT: usize = 1;

#[test]
fn exact_pipe_proof_budget_excludes_unqueried_descriptor_types() {
    let descriptors = vec![
        descriptor(FIRST_NON_PIPE_DESCRIPTOR, libc::PROX_FDTYPE_VNODE),
        descriptor(PIPE_DESCRIPTOR, libc::PROX_FDTYPE_PIPE),
        descriptor(SECOND_NON_PIPE_DESCRIPTOR, libc::PROX_FDTYPE_SOCKET),
    ];
    let mut budget = CapabilityScanBudget::start_batch();

    let admitted = admit_pipe_descriptor_work(descriptors, &mut budget).unwrap();

    assert_eq!(admitted.len(), EXPECTED_PIPE_DESCRIPTOR_COUNT);
    assert_eq!(admitted[0].proc_fd, PIPE_DESCRIPTOR);
    assert_eq!(budget.descriptor_count(), EXPECTED_PIPE_DESCRIPTOR_COUNT);
}

fn descriptor(raw: i32, descriptor_type: i32) -> libc::proc_fdinfo {
    libc::proc_fdinfo {
        proc_fd: raw,
        proc_fdtype: descriptor_type as u32,
    }
}
