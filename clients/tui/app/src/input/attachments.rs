use crate::{Mode, State, chat, network::Job};
use crossterm::event::Event;
use tokio::sync::mpsc;
use yolo_hub_identity::{AgentTarget, RuntimeTarget};
use yolo_tui_clipboard::{Dialog, InitialSource, ReferenceStyle};

#[derive(PartialEq)]
struct Destination {
    pane: chat::Key,
    lens: Mode,
    runtime: RuntimeTarget,
    agent: Option<AgentTarget>,
}

impl Destination {
    fn current(state: &State) -> Option<Self> {
        let terminal = state.terminals.get(&state.focused_terminal()?)?.fields();
        let runtime = terminal.runtime?.target;
        match state.mode {
            Mode::Chat => {
                let view = chat::current(state)?;
                if view.retired()
                    || view.composer.locked()
                    || view.binding().runtime != runtime
                    || Some(view.binding().agent) != terminal.agent
                {
                    return None;
                }
            }
            Mode::Terminal => {}
            _ => return None,
        }
        Some(Self {
            pane: chat::Key::current(state),
            lens: state.mode,
            runtime,
            agent: terminal.agent,
        })
    }
}

pub(crate) struct Attachment {
    destination: Destination,
    pub dialog: Dialog,
}

pub(crate) fn open(state: &mut State, source: InitialSource) {
    let Some(destination) = Destination::current(state) else {
        state.status = "Select a live shell or an editable agent Chat to attach a file".into();
        return;
    };
    let style = if state.mode == Mode::Chat {
        ReferenceStyle::Chat
    } else {
        ReferenceStyle::Shell
    };
    let dialog = Dialog::new(destination.runtime, source, style);
    state.attachment = Some(Attachment {
        destination,
        dialog,
    });
}

pub(crate) fn edit(state: &mut State, event: &Event) -> bool {
    let Some(attachment) = state.attachment.as_mut() else {
        return false;
    };
    if attachment.dialog.dismiss(event) {
        state.attachment = None;
    }
    // Let geometry and outer-terminal focus events continue through the normal router.
    matches!(event, Event::Key(_) | Event::Paste(_) | Event::Mouse(_))
}

pub(crate) fn complete(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    result: anyhow::Result<String>,
) {
    let Some(attachment) = state.attachment.take() else {
        return;
    };
    if Destination::current(state) != Some(attachment.destination) {
        state.status = "Attachment target changed".into();
        return;
    }
    match result {
        Ok(path) => {
            if state.mode == Mode::Chat {
                chat::handle(state, jobs, &Event::Paste(path));
            } else {
                crate::terminal_actions::paste(state, jobs, &path);
            }
        }
        Err(error) => state.status = format!("Could not attach: {error:#}"),
    }
}
