use super::*;

impl PendingBroker {
    pub(super) fn supervised(supervised: SupervisedBroker) -> Self {
        Self {
            detached: None,
            supervised: Some(supervised),
            #[cfg(test)]
            observer: None,
            #[cfg(test)]
            compensation: None,
        }
    }

    pub fn matches_status(&self, status: &yolo_remote_broker::BrokerStatus) -> bool {
        self.supervised
            .as_ref()
            .is_none_or(|supervised| supervised.matches_status(status))
    }

    pub fn detached(child: DetachedChildCustody, termination: TerminationReservation) -> Self {
        Self {
            detached: Some(DetachedBroker { child, termination }),
            supervised: None,
            #[cfg(test)]
            observer: None,
            #[cfg(test)]
            compensation: None,
        }
    }

    pub fn observe(&mut self) -> Result<PendingObservation> {
        if let Some(supervised) = self.supervised.as_mut() {
            let Some(status) = supervised.observe()? else {
                return Ok(PendingObservation::Pending);
            };
            self.supervised.take();
            return Ok(PendingObservation::Exited(status.to_string()));
        }
        if let Some(detached) = self.detached.as_mut() {
            let Some(status) = observe_child(&mut detached.child)? else {
                return Ok(PendingObservation::Pending);
            };
            self.detached.take();
            return Ok(PendingObservation::Exited(status.to_string()));
        }
        #[cfg(test)]
        if let Some(observer) = self.observer.as_mut() {
            return observer();
        }
        Ok(PendingObservation::Pending)
    }

    pub fn adopt(self) -> Result<()> {
        self.handoff()
    }
}
