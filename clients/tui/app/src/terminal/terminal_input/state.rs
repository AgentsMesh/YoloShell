impl Inputs {
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub(crate) fn scope_current(&self, state: &State) -> bool {
        self.scope
            .as_ref()
            .is_some_and(|scope| scope.current(state))
    }

    pub fn can_read(&self) -> bool {
        !self.closed && self.waiting.is_none() && self.pending.len() < MAX_PENDING_INPUTS
    }

    pub(crate) fn dispatched_len(&self) -> usize {
        self.pending
            .len()
            .saturating_sub(self.deferred.len())
            .saturating_sub(usize::from(self.waiting.is_some()))
    }

    pub(crate) fn unsent_len(&self) -> usize {
        self.pending.len().saturating_sub(self.dispatched_len())
    }

    pub fn waiting(&self) -> bool {
        self.waiting.is_some()
    }

    pub fn matches_scope(&self, request: &ControlRequest) -> bool {
        self.scope.as_ref().is_some_and(|scope| {
            request.validity.as_ref().is_some_and(|validity| {
                validity.same_scope(&scope.queue.validity_until(validity.deadline()))
            })
        })
    }

    pub(super) fn pending_for(&self, address: TerminalAddress) -> bool {
        self.pending
            .values()
            .any(|request| input_request_address(request) == Some(address))
    }

    pub(crate) fn settle_pending(&mut self, id: Uuid) -> bool {
        let Some(_request) = self.pending.remove(&id) else {
            return false;
        };
        true
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.scope = None;
        self.streams.clear();
        self.waiting = None;
        while let Some(deferred) = self.deferred.pop_front() {
            deferred.retire();
        }
        self.overflowed.clear();
        self.pending.clear();
        self.interrupted.clear();
        self.resume_requested.clear();
        self.failures.clear();
        self.pregrant.clear();
        self.snapshot_pending.clear();
    }

    pub fn retire_writer(&mut self, address: TerminalAddress) {
        if let Some(stream) = self.streams.get(&address)
            && self.pending_in_stream(stream)
        {
            self.interrupted.insert(address, stream.grant);
            self.resume_requested.remove(&address);
        } else {
            self.failures.remove(&address);
        }
        if self
            .scope
            .as_ref()
            .is_some_and(|scope| scope.grant.fields().target.address == address)
        {
            self.scope.take();
        }
    }

    pub(crate) fn discard_writer(&mut self, address: TerminalAddress) {
        self.pregrant
            .retain(|item| item.target != pregrant::PregrantTarget::Address(address));
        self.snapshot_pending.remove(&address);
        let mut deferred = VecDeque::new();
        while let Some(item) = self.deferred.pop_front() {
            if item.address == address {
                item.retire();
                self.pending.remove(&input_request_id(&item.request));
            } else {
                deferred.push_back(item);
            }
        }
        self.deferred = deferred;
        if self
            .waiting
            .as_ref()
            .and_then(input_request_address)
            .is_some_and(|target| target == address)
        {
            if let Some(request) = self.waiting.take() {
                if let Some(validity) = request.validity.as_ref() {
                    validity.reject();
                }
                self.pending.remove(&input_request_id(&request));
            }
        }
        self.pending.retain(|_, request| {
            if input_request_address(request) == Some(address) {
                if let Some(validity) = &request.validity {
                    validity.reject();
                }
                false
            } else {
                true
            }
        });
        self.streams.remove(&address);
        self.interrupted.remove(&address);
        self.resume_requested.remove(&address);
        self.failures.remove(&address);
        self.overflowed.remove(&address);
        if self
            .scope
            .as_ref()
            .is_some_and(|scope| scope.grant.fields().target.address == address)
        {
            self.scope.take();
        }
    }

    pub(super) fn pending_in_scope(&self, scope: &Scope) -> bool {
        self.pending.values().any(|request| {
            request.validity.as_ref().is_some_and(|validity| {
                validity.same_scope(&scope.queue.validity_until(validity.deadline()))
            })
        })
    }

    pub(super) fn pending_in_stream(&self, stream: &crate::terminal_input_stream::Stream) -> bool {
        self.pending.values().any(|request| stream.matches(request))
    }

    pub(crate) fn resume_overflow(&mut self, address: TerminalAddress) -> bool {
        let overflowed = self.overflowed.remove(&address);
        let queued = self.pending.values().any(|request| {
            matches!(&request.command, Command::Input { request }
                if request.fields().grant.fields().target.address == address)
        }) || self
            .deferred
            .iter()
            .any(|deferred| deferred.address == address);
        overflowed && queued
    }

    pub(super) fn matches_stream(&self, generation: Option<Uuid>, grant: LeaseGrant) -> bool {
        self.streams
            .get(&grant.fields().target.address)
            .is_some_and(|stream| stream.same_authority(generation, grant))
    }
}

fn input_request_address(request: &ControlRequest) -> Option<TerminalAddress> {
    match &request.command {
        Command::Input { request } => Some(request.fields().grant.fields().target.address),
        _ => None,
    }
}

fn input_request_id(request: &ControlRequest) -> Uuid {
    let Command::Input { request } = &request.command else {
        unreachable!("input state contains a non-input request")
    };
    request.fields().request_id
}
