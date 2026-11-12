pub async fn run_with_startup(
    options: Options,
    shutdown: impl std::future::Future<Output = ()>,
    action: crate::StartupAction,
) -> anyhow::Result<()> {
    tokio::pin!(shutdown);
    options.endpoint.validate()?;
    let capabilities = tokio::select! {
        _ = &mut shutdown => return Ok(()),
        result = crate::client_capabilities::open(&options) => result?,
    };
    let mut tab_store = crate::tab_persistence::open(&options)?;
    let mut control = yolo_tui_client_control::ClientControl::bind(&tab_store).await?;
    let mut state = crate::tab_persistence::restore(&tab_store);
    state.client_capabilities = Some(capabilities);
    let mut tab_saved = crate::tab_persistence::SaveState::default();
    crate::layout_sync::startup::open(&options, &mut tab_store, &mut state).await?;
    if let Some(client_id) = options.endpoint.client_id() {
        state.principal.client_id = client_id;
    }
    crate::finalization_persistence::open(&mut state, &tab_store, options.hub)?;
    crate::startup::install(&mut state, options.hub, action);
    let local_host = options
        .endpoint
        .local_socket()
        .and_then(|_| yolo_tui_local_startup::local_hostname());
    state.inventory.set_local_hub(options.hub, local_host);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut hangup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut quit = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::quit())?;
    let mut screen = ScreenGuard::enter()?;
    let mut input = EventStream::new();
    let (jobs, receiver) = mpsc::channel(QUEUE_CAPACITY);
    let (updates, mut responses) = mpsc::channel(QUEUE_CAPACITY);
    let (priority_updates, mut priority_responses) =
        mpsc::channel(QUEUE_CAPACITY + DEFERRED_INPUT_CAPACITY);
    let mut priority_closed = false;
    let mut yield_after_priority = false;
    let (observations, mut observation_updates) = mpsc::channel(QUEUE_CAPACITY);
    let mut worker = tokio::spawn(network::network_with_priority(
        options.clone(),
        receiver,
        updates.clone(),
        priority_updates.clone(),
        true,
    ));
    let mut observers = crate::observers::Observers::default();
    let mut ticks = interval(EVENT_POLL);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut catalog = interval(CATALOG_REFRESH);
    catalog.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut synchronized_hold = None;
    let result = async {
        loop {
            crate::layout_sync::pump(&mut state, &jobs);
            state.sync_active_tab_view();
            crate::chat::pump(&mut state, &jobs);
            crate::startup::apply(&mut state, &jobs);
            crate::selection_actions::sync(&mut state);
            if state.mode == crate::Mode::Terminal {
                if let Some(address) = state.focused_terminal() {
                    state.inventory.attention.remove(&address);
                }
            }
            if let Some(bell) = state.notifications.take_bell() {
                screen.notify(bell)?;
            }
            crate::tab_persistence::save_if_changed(&mut tab_store, &state, &mut tab_saved)?;
            if let Some(command) = crate::selection_actions::take_clipboard(&mut state) {
                screen.copy(&command)?;
            }
            let synchronized = synchronized_terminal(&state);
            let now = Instant::now();
            let hold = if synchronized {
                let started = *synchronized_hold.get_or_insert(now);
                if now.duration_since(started) < SYNCHRONIZED_OUTPUT_HOLD_LIMIT {
                    true
                } else {
                    // Surface a frame at most once per safety window, then
                    // resume holding. Drawing on every loop would reintroduce
                    // the intermediate cursor movement this mechanism avoids.
                    synchronized_hold = Some(now);
                    false
                }
            } else {
                synchronized_hold = None;
                false
            };
            if !hold {
                let mut presentation = crate::presentation::Presentation::default();
                screen.terminal.draw(|frame| presentation = draw::draw(frame, &state))?;
                state.presentation = presentation;
                if let Some(view) = state.presentation.panes.iter().find(|view| view.pane == state.tab_layout().focused()).copied() {
                    crate::resize::apply(&mut state, &jobs, view);
                }
            }
            state.sync_query_authority();
            crate::terminal_input::drain_pregrant(&mut state, &jobs);
            let allow_priority = !yield_after_priority;
            yield_after_priority = false;
            tokio::select! {
                biased;
                priority = priority_responses.recv(), if allow_priority && !priority_closed => match priority {
                    Some(update) => {
                        events::apply(&mut state, &jobs, update);
                        yield_after_priority = true;
                    }
                    None => priority_closed = true,
                },
                _ = terminate.recv() => break,
                _ = hangup.recv() => break,
                _ = interrupt.recv() => break,
                _ = quit.recv() => break,
                _ = &mut shutdown => break,
                result = control.detached() => { result?; break; },
                event = input.next() => match event {
                    Some(Ok(event)) => if actions::handle(&mut state, &jobs, options.hub, event) { break; },
                    Some(Err(error)) => return Err(anyhow::Error::from(error)),
                    None => break,
                },
                permit = jobs.reserve(), if state.inputs.waiting() || crate::terminal_input::deferred_ready(&state) => match permit {
                    Ok(permit) => crate::terminal_input::flush(&mut state, permit),
                    Err(_) => break,
                },
                update = responses.recv() => match update { Some(update) => events::apply(&mut state, &jobs, update), None => break },
                update = observation_updates.recv() => match update { Some(update) => events::apply(&mut state, &jobs, update), None => break },
                _ = ticks.tick() => {
                    crate::polling::renew_leases(&mut state, &jobs);
                    observers.sync(&options, &mut state, &observations);
                },
                _ = catalog.tick() => {
                    events::refresh(&mut state, &jobs);
                    crate::control::refresh_observations(&mut state, &jobs);
                    crate::layout_sync::refresh(&mut state);
                },
            }
            crate::update_queue::drain_bounded(
                &mut state,
                &jobs,
                &mut priority_responses,
                &mut responses,
                &mut observation_updates,
                crate::constants::MAX_COALESCED_UPDATES,
            );
            if let Some(message) = state.client_startup_failure.take() {
                return Err(anyhow::anyhow!(message));
            }
            state.sync_query_authority();
        }
        Ok(())
    }.await;
    state.inputs.close();
    state.queries.scopes.clear();
    observers.close().await;
    drop(updates);
    drop(priority_updates);
    drop(observations);
    // Closing the view never terminates its shells. Leases expire if release cannot be delivered.
    for (address, writer) in std::mem::take(&mut state.writers).into_iter().filter(|_| {
        state.client_capability_enabled(
            yolo_builtin_client_contract::ClientCapability::TerminalControl,
        )
    }) {
        let _ = jobs.try_send(Job::Call {
            generation: state.control_generation,
            origin: address.hub_id,
            command: Command::Release {
                grant: writer.grant,
            },
        });
    }
    drop(jobs);
    drop(responses);
    drop(observation_updates);
    if tokio::time::timeout(SHUTDOWN_GRACE, &mut worker)
        .await
        .is_err()
    {
        worker.abort();
        let _ = worker.await;
    }
    drop(screen);
    let result = result.and(crate::tab_persistence::save(&mut tab_store, &state));
    let result = result.and(control.withdraw());
    drop(state);
    drop(tab_store);
    result?;
    control.finish().await
}
