use super::*;

impl WorkerState {
    pub(super) async fn write_input(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &InputRequest,
    ) -> std::result::Result<InputResult, ErrorBody> {
        let permit = self
            .admit_writer_mutation(controller, request.generation, &request.writer_capability)
            .await?;
        #[cfg(test)]
        self.control_test_hooks
            .input_before_delivery
            .pause_if_armed()
            .await;
        self.write_input_admitted(&permit, request)
    }

    pub(super) fn write_input_admitted(
        &self,
        _permit: &WriterMutationPermit,
        request: &InputRequest,
    ) -> std::result::Result<InputResult, ErrorBody> {
        if request.data.is_empty() {
            return Err(ErrorBody::invalid(TERMINAL_INPUT_EMPTY_DETAIL));
        }
        let mut delivered = lock_recover(&self.delivered_inputs);
        match delivered.get(&request.writer_capability, request.input_id, &request.data) {
            CachedInputLookup::Matching(CachedInputDelivery::Delivered) => {
                return Ok(InputResult {
                    input_id: request.input_id,
                    duplicate: true,
                    delivery: crate::protocol::delivery::DELIVERED.into(),
                });
            }
            CachedInputLookup::Matching(CachedInputDelivery::Unknown) => {
                return Err(input_delivery_unknown(request.input_id, None));
            }
            CachedInputLookup::PayloadMismatch => {
                return Err(ErrorBody::conflict(TERMINAL_INPUT_ID_REUSE_DETAIL));
            }
            CachedInputLookup::Missing => {}
        }
        let mut writer = lock_recover(&self.writer);
        let writer = writer
            .as_mut()
            .ok_or_else(|| ErrorBody::conflict(TERMINAL_WRITER_CLOSED_DETAIL))?;
        if !delivered.reserve(
            &request.writer_capability,
            request.input_id,
            &request.data,
            DELIVERED_INPUT_CACHE_LIMIT,
        ) {
            return Err(ErrorBody::conflict(
                input_delivery::CAPACITY_EXHAUSTED_DETAIL,
            ));
        }
        let write = self.write_terminal_input_once(writer, &request.data);
        if let Err(error) = write {
            return Err(input_delivery_unknown(request.input_id, Some(error)));
        }
        delivered.mark_delivered(&request.writer_capability, request.input_id);
        Ok(InputResult {
            input_id: request.input_id,
            duplicate: false,
            delivery: crate::protocol::delivery::DELIVERED.into(),
        })
    }

    pub(super) fn write_terminal_input_once(&self, writer: &mut dyn Write, data: &[u8]) -> io::Result<()> {
        #[cfg(unix)]
        {
            let master = lock_recover(&self.master);
            let master = master.as_ref().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, TERMINAL_PTY_CLOSED_DETAIL)
            })?;
            configure_nonblocking_pty(master.as_ref())?;
        }
        let written = writer.write(data)?;
        if written != data.len() {
            return Err(io::Error::other(PTY_INPUT_INCOMPLETE_DETAIL));
        }
        Ok(())
    }

    pub(super) async fn resize_terminal(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &ResizeRequest,
    ) -> std::result::Result<(), ErrorBody> {
        if request.cols == 0 || request.rows == 0 {
            return Err(ErrorBody::invalid(TERMINAL_DIMENSIONS_INVALID_DETAIL));
        }
        let permit = self
            .admit_writer_mutation(controller, request.generation, &request.writer_capability)
            .await?;
        self.resize_terminal_admitted(&permit, request)
    }

    pub(super) fn resize_terminal_admitted(
        &self,
        _permit: &WriterMutationPermit,
        request: &ResizeRequest,
    ) -> std::result::Result<(), ErrorBody> {
        let cols = normalize_terminal_columns(request.cols);
        let rows = normalize_terminal_rows(request.rows);
        {
            let master = lock_recover(&self.master);
            master
                .as_ref()
                .ok_or_else(|| ErrorBody::conflict(TERMINAL_PTY_CLOSED_DETAIL))?
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(internal_error)?;
        }
        let mut dimensions = lock_recover(&self.dimensions);
        let previous = *dimensions;
        *dimensions = (cols, rows);
        #[cfg(test)]
        self.control_test_hooks
            .resize_after_dimensions
            .pause_if_armed();
        lock_recover(&self.terminal)
            .screen_mut()
            .set_size(rows, cols);
        drop(dimensions);
        let mut record = lock_recover(&self.record);
        let mut next = record.clone();
        next.cols = cols;
        next.rows = rows;
        if let Err(error) = self.persist_runtime_record(&next)
            && let Err(rejected) =
                self.reconcile_runtime_write(&next, error, TERMINAL_RUNTIME_PERSISTENCE_DETAIL)
        {
            // The runtime record is the geometry authority. A failed publication
            // cannot leave the live PTY and parser reporting a size the durable
            // record never accepted.
            self.restore_terminal_size(previous);
            return Err(rejected);
        }
        *record = next;
        Ok(())
    }

    pub(super) fn restore_terminal_size(&self, (cols, rows): (u16, u16)) {
        let resized = lock_recover(&self.master).as_ref().map(|master| {
            master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
        });
        if let Some(Err(error)) = resized {
            tracing::error!(%error, "restore terminal geometry after rejected resize");
        }
        // The snapshot reports dimensions and screen contents together, so
        // the parser resize must happen under the same dimensions lock.
        let mut dimensions = lock_recover(&self.dimensions);
        *dimensions = (cols, rows);
        lock_recover(&self.terminal)
            .screen_mut()
            .set_size(rows, cols);
        drop(dimensions);
    }
}
