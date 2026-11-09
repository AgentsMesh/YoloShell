impl FinalizerTestProcess {
    fn settle_naturally(&mut self) -> bool {
        let deadline = Instant::now() + FINALIZER_PROCESS_REAP_TIMEOUT;
        while Instant::now() < deadline {
            if self.refresh_containment() {
                return true;
            }
            std::thread::sleep(FINALIZER_PROCESS_POLL_INTERVAL);
        }
        false
    }

    fn fail(&mut self, detail: &str) -> ! {
        if self.terminate_and_reap() {
            panic!("{detail}");
        }
        panic!("{detail}; {FINALIZER_PROCESS_REAP_DETAIL}");
    }

    fn terminate_and_reap(&mut self) -> bool {
        let _ = self.refresh_containment();
        if let Some(containment) = self.containment.as_mut() {
            let _ = containment.terminate(libc::SIGKILL);
        }
        self.terminate_private_group();
        if let Some(child) = self.child.as_mut()
            && !matches!(child.request_force_termination(), Ok(true))
        {
            let _ = child.request_direct_termination();
        }
        let deadline = Instant::now() + FINALIZER_PROCESS_REAP_TIMEOUT;
        loop {
            let tree_settled = self.refresh_containment();
            let child_settled = self.reap_child();
            if tree_settled && child_settled {
                self.settled = true;
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(FINALIZER_PROCESS_POLL_INTERVAL);
        }
    }

    fn refresh_containment(&mut self) -> bool {
        let Some(containment) = self.containment.as_mut() else {
            return true;
        };
        if containment.reconcile_retained_root_exit_for_test().is_err() {
            return false;
        }
        let Ok(live) = containment.observe_live_identities_for_test() else {
            return false;
        };
        if !live.is_empty() || containment.reconcile_retained_exact_empty_for_test().is_err() {
            return false;
        }
        containment.descendant_observation_for_test() == crate::process::DescendantObservation::Settled
    }

    fn reap_child(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            self.child.take();
            return true;
        }
        false
    }

    fn terminate_private_group(&self) {
        let Some(identity) = self.identity else {
            return;
        };
        if identity.observe() != crate::process::ProcessIdentityObservation::Matching {
            return;
        }
        let Ok(group_id) = i32::try_from(identity.pid) else {
            return;
        };
        if let Ok(scope) = crate::process::PtyProcessScope::capture(identity)
            && let Ok(group) = scope.foreground_group(group_id)
        {
            let _ = crate::process::signal_process_group(group, libc::SIGKILL);
        }
    }
}

impl Drop for FinalizerTestProcess {
    fn drop(&mut self) {
        if !self.settled {
            let _ = self.terminate_and_reap();
        }
    }
}
