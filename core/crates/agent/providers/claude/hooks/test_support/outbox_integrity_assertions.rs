use super::*;
use std::cell::Cell;

impl Fixture {
    pub fn reject_corruption(&self, label: &str, damaged: &OutboxState) {
        let encoded = serde_json::to_vec(damaged).unwrap();
        assert_ne!(&self.bytes(), &encoded, "{label}");
        std::fs::write(&self.path, &encoded).unwrap();
        let inspected = Cell::new(false);
        self.rejected(
            persistence::read(&self.path, |_| {
                inspected.set(true);
                Ok(())
            }),
            &encoded,
            label,
        );
        assert!(!inspected.get(), "{label}");
        let mutated = Cell::new(false);
        self.rejected(
            persistence::update(&self.path, |_| {
                mutated.set(true);
                Ok(())
            }),
            &encoded,
            label,
        );
        assert!(!mutated.get(), "{label}");
        self.rejected(next(&self.path, self.delivery), &encoded, label);
        self.rejected(
            probe_recovery(&self.path, self.credentials),
            &encoded,
            label,
        );
        self.rejected(
            enqueue(
                &self.path,
                self.credentials,
                events::state_changed(&self.session),
            ),
            &encoded,
            label,
        );
        let replayed = Cell::new(false);
        self.rejected(
            fence_session_start_replay(
                &self.path,
                self.credentials,
                self.accepted.event_sequence,
                self.accepted.event_id,
                self.agent,
                |_| {
                    replayed.set(true);
                    Ok(())
                },
            ),
            &encoded,
            label,
        );
        assert!(!replayed.get(), "{label}");
        let confirmed = Cell::new(false);
        self.rejected(
            fence_authenticated_session_start(
                &self.path,
                self.credentials.invocation_id,
                self.agent,
                |_| {
                    confirmed.set(true);
                    Ok(())
                },
            ),
            &encoded,
            label,
        );
        assert!(!confirmed.get(), "{label}");
        self.rejected(self.prepare(), &encoded, label);
        self.rejected(self.commit(), &encoded, label);
        let reconciled = Cell::new(false);
        self.rejected(
            reconcile_prepared_session_starts(&self.path, |_| {
                reconciled.set(true);
                Ok(Some(events::after()))
            }),
            &encoded,
            label,
        );
        assert!(!reconciled.get(), "{label}");
    }

    fn rejected<T>(&self, result: anyhow::Result<T>, encoded: &[u8], label: &str) {
        match result {
            Ok(_) => panic!("{label}: corrupted outbox was accepted"),
            Err(error) => assert_eq!(error.to_string(), outbox_error::STATE_INVALID, "{label}"),
        }
        assert_eq!(self.bytes(), encoded, "{label}");
    }
}
