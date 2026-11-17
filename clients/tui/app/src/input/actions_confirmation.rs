fn handle_confirmation(state: &mut State, jobs: &mpsc::Sender<Job>, key: KeyEvent) {
    if key.code == KeyCode::Enter {
        if let Some(command) = state.confirmation.take() {
            enqueue(
                state,
                jobs,
                Job::Call {
                    generation: None,
                    origin: command.selection.address.hub_id,
                    command: Command::Terminal { command },
                },
            );
        }
    }
    state.confirmation = None;
    state.mode = Mode::Inventory;
}
