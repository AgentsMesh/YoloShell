use super::super::{FaultOutput, FaultReap};
use super::progress::PROCESS_CUSTODY_MISSING_DETAIL;
use crate::process::WaitAuthorityLoss;
use anyhow::{Context, Result};

pub(crate) fn reconcile_root_exit_proof(ownership: &mut FaultReap) -> Result<()> {
    let Some(proof) = ownership.custody().root_exit_proof else {
        return Ok(());
    };
    ownership
        .custody_mut()
        .processes
        .as_mut()
        .context(PROCESS_CUSTODY_MISSING_DETAIL)?
        .mark_root_exited_unreaped(proof)
}

pub(crate) fn observe_root_exit(ownership: &mut FaultReap) {
    if ownership.custody().root_exit_proof.is_none() && ownership.custody().authority.is_some() {
        observe_waitable_root_exit(ownership);
    } else {
        wait_observed_root_exit(ownership);
    }
    observe_lost_root_gone(ownership);
    if let Err(error) = reconcile_root_exit_proof(ownership) {
        ownership.remember(Err(error));
    }
}

pub(crate) fn observe_waitable_root_exit(ownership: &mut FaultReap) {
    let observed = ownership
        .custody()
        .authority
        .as_ref()
        .unwrap()
        .poll_exited_unreaped();
    match observed {
        Ok(Some(proof)) => {
            let result = ownership
                .custody_mut()
                .processes
                .as_mut()
                .expect(PROCESS_CUSTODY_MISSING_DETAIL)
                .mark_root_exited_unreaped(proof);
            match result {
                Ok(()) => ownership.custody_mut().root_exit_proof = Some(proof),
                Err(error) => ownership.remember(Err(error)),
            }
        }
        Ok(None) => {}
        Err(error) => retain_or_lose_wait_authority(ownership, error),
    }
    wait_observed_root_exit(ownership);
}

pub(crate) fn wait_observed_root_exit(ownership: &mut FaultReap) {
    let custody = ownership.custody_mut();
    let (Some(proof), Some(authority)) = (custody.root_exit_proof, custody.authority.as_mut())
    else {
        return;
    };
    let waited = authority.wait_exited(proof);
    match waited {
        Ok(_) => publish_direct_child_waited(ownership),
        Err(error) => retain_or_lose_wait_authority(ownership, error),
    }
}

pub(crate) fn publish_direct_child_waited(ownership: &FaultReap) {
    if let FaultOutput::Reader(output) = &ownership.custody().output_progress {
        output.recovery().publish_direct_child_waited();
    }
}

pub(crate) fn retain_or_lose_wait_authority(ownership: &mut FaultReap, error: anyhow::Error) {
    let loss =
        crate::worker::supervision::contain(|| WaitAuthorityLoss::from_error(&error)).flatten();
    if let Some(loss) = loss {
        ownership.remember_terminal(error);
        ownership.lose_wait_authority(loss);
    } else {
        ownership.remember(Err(error));
    }
}

pub(crate) fn observe_lost_root_gone(ownership: &mut FaultReap) {
    if ownership.root_gone_without_wait {
        return;
    }
    let Some(authority) = ownership.lost_wait_custody.as_ref() else {
        return;
    };
    let proof = match authority.observe_gone() {
        Ok(Some(proof)) => proof,
        Ok(None) => return,
        Err(error) => {
            ownership.remember(Err(error));
            return;
        }
    };
    let validation = authority.validate(&proof);
    if let Err(error) = validation.and_then(|()| {
        ownership
            .custody_mut()
            .processes
            .as_mut()
            .expect(PROCESS_CUSTODY_MISSING_DETAIL)
            .mark_root_gone_without_wait(proof)
    }) {
        ownership.remember(Err(error));
        return;
    }
    ownership.root_gone_without_wait = true;
}
