use super::*;

pub(crate) fn dispatch(slots: Arc<Mutex<Vec<Arc<EmergencySlot>>>>) -> ! {
    let mut retry_delay = super::super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL;
    loop {
        if dispatch_ready(&slots) {
            std::thread::park_timeout(retry_delay);
            retry_delay = retry_delay
                .saturating_mul(super::super::constants::PTY_FAULT_REAPER_RETRY_MULTIPLIER)
                .min(super::super::constants::PTY_FAULT_REAPER_RETRY_MAX_INTERVAL);
        } else {
            retry_delay = super::super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL;
            std::thread::park();
        }
    }
}

pub(crate) fn dispatch_ready(slots: &Mutex<Vec<Arc<EmergencySlot>>>) -> bool {
    let ready = {
        let mut slots = lock(slots);
        slots.retain(|slot| !slot.phase().is_terminal());
        slots
            .iter()
            .filter(|slot| slot.phase() == EmergencyPhase::Ready)
            .cloned()
            .collect::<Vec<_>>()
    };
    ready.iter().fold(false, |retry, slot| {
        (!spawn_ready(slot) && slot.phase() == EmergencyPhase::Ready) || retry
    })
}

pub(crate) fn spawn_ready(slot: &Arc<EmergencySlot>) -> bool {
    if slot
        .phase
        .compare_exchange(
            EmergencyPhase::Ready as u8,
            EmergencyPhase::Driving as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return true;
    }
    let spawn = catch_unwind(AssertUnwindSafe(|| spawn_owner(slot)));
    match spawn {
        Ok(Ok(_)) => true,
        Ok(Err(error)) => {
            remember_owner_spawn_failure(
                slot,
                anyhow::Error::new(error).context(OWNER_SPAWN_FAILURE_DETAIL),
            );
            restore_ready(slot);
            #[cfg(test)]
            slot.test_hooks.record_owner_spawn_failure_and_wait();
            false
        }
        Err(payload) => {
            let detail = crate::worker::supervision::panic_detail(payload);
            remember_owner_spawn_failure(
                slot,
                anyhow::anyhow!("{OWNER_SPAWN_PANIC_DETAIL}: {detail}"),
            );
            restore_ready(slot);
            #[cfg(test)]
            slot.test_hooks.record_owner_spawn_failure_and_wait();
            false
        }
    }
}

pub(crate) fn spawn_owner(
    slot: &Arc<EmergencySlot>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let owned = Arc::clone(slot);
    #[cfg(test)]
    slot.test_hooks.inject_owner_spawn_fault()?;
    std::thread::Builder::new()
        .name(OWNER_THREAD_NAME.to_owned())
        .spawn(move || drive(owned))
}

pub(crate) fn restore_ready(slot: &EmergencySlot) {
    slot.phase
        .store(EmergencyPhase::Ready as u8, Ordering::Release);
}

pub(crate) fn drive(slot: Arc<EmergencySlot>) {
    // SAFETY: the owner exclusively holds the Driving phase and Ready/Release
    // published a fully initialized envelope.
    let envelope = unsafe { (*slot.envelope.get()).assume_init_mut() };
    refresh_schedule(&mut envelope.reap);
    #[cfg(test)]
    slot.test_hooks
        .record_drive_schedule(envelope.reap.schedule.is_prompt_for_test());
    loop {
        match catch_unwind(AssertUnwindSafe(|| drive_once(&slot))) {
            Ok(()) => break,
            Err(payload) => remember_driver_panic(&slot, payload),
        }
    }
    // SAFETY: this is the sole Driving consumer and the envelope was initialized
    // before Ready/Release. No other access is allowed until Settled publication.
    let _ = crate::worker::supervision::contain(|| unsafe {
        std::ptr::drop_in_place((*slot.envelope.get()).as_mut_ptr())
    });
    slot.phase
        .store(EmergencyPhase::Settled as u8, Ordering::Release);
    slot.dispatcher.unpark();
}

pub(crate) fn refresh_schedule(reap: &mut FaultReap) {
    reap.schedule = super::super::schedule::FaultReapSchedule::production();
}

pub(crate) fn drive_once(slot: &EmergencySlot) {
    // SAFETY: only the successful Ready->Driving claimant reaches this function.
    let envelope = unsafe { (*slot.envelope.get()).assume_init_mut() };
    envelope.reap.publish_handoff();
    if let Some(payload) = envelope.panic.take() {
        let detail = crate::worker::supervision::panic_detail(payload);
        envelope
            .reap
            .remember(Err(anyhow::anyhow!("{DROP_PANIC_DETAIL}: {detail}")));
    } else if envelope.reap.failure.is_none() {
        envelope
            .reap
            .remember(Err(anyhow::anyhow!(OWNERSHIP_DROP_DETAIL)));
    }
    envelope.reap.reap_retained();
}

pub(crate) fn remember_owner_spawn_failure(slot: &EmergencySlot, error: anyhow::Error) {
    let _ = crate::worker::supervision::contain(move || {
        // SAFETY: spawn failure is handled while this dispatcher owns Driving.
        let envelope = unsafe { (*slot.envelope.get()).assume_init_mut() };
        envelope.reap.publish_handoff();
        envelope.reap.remember(Err(error));
    });
}

pub(crate) fn remember_driver_panic(slot: &EmergencySlot, payload: Box<dyn Any + Send>) {
    let detail = crate::worker::supervision::panic_detail(payload);
    let _ = crate::worker::supervision::contain(|| {
        // SAFETY: the owner thread retains exclusive Driving custody.
        let envelope = unsafe { (*slot.envelope.get()).assume_init_mut() };
        envelope
            .reap
            .remember(Err(anyhow::anyhow!("{DRIVER_PANIC_DETAIL}: {detail}")));
    });
    std::thread::sleep(super::super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL);
}

impl EmergencySlot {
    pub(crate) fn phase(&self) -> EmergencyPhase {
        match self.phase.load(Ordering::Acquire) {
            phase if phase == EmergencyPhase::Vacant as u8 => EmergencyPhase::Vacant,
            phase if phase == EmergencyPhase::Ready as u8 => EmergencyPhase::Ready,
            phase if phase == EmergencyPhase::Driving as u8 => EmergencyPhase::Driving,
            phase if phase == EmergencyPhase::Cancelled as u8 => EmergencyPhase::Cancelled,
            _ => EmergencyPhase::Settled,
        }
    }
}

impl EmergencyPhase {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Settled | Self::Cancelled)
    }
}
