use crate::{
    MAX_NOTIFICATION_BODY_BYTES, MAX_NOTIFICATION_TITLE_BYTES, MAX_TITLE_BYTES, NotificationKind,
    TerminalNotification, TerminalPresentation, constants::*, text,
};

/// Presentation metadata belongs to a continuously parsed, single runtime PTY.
/// It never grants filesystem, agent lifecycle, or control authority.
#[derive(Default)]
pub struct MetadataCallbacks {
    presentation: TerminalPresentation,
}

impl MetadataCallbacks {
    pub fn new(presentation: TerminalPresentation) -> Self {
        Self { presentation }
    }

    pub fn presentation(&self) -> &TerminalPresentation {
        &self.presentation
    }

    /// Reconstruct the title and the alternate-screen mode. Historical alerts
    /// must not ring on attachment; a snapshot of the alternate grid must not
    /// be replayed into the primary grid.
    pub fn snapshot_prefix(&self, alternate_screen: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        if alternate_screen {
            bytes.extend_from_slice(ALTERNATE_SCREEN_ENTER);
        }
        if let Some(title) = &self.presentation.title {
            bytes.extend_from_slice(TITLE_PREFIX);
            bytes.extend_from_slice(title.as_bytes());
            bytes.extend_from_slice(STRING_TERMINATOR);
        }
        bytes
    }

    fn title(&mut self, parts: &[&[u8]]) {
        self.presentation.title = Some(text::sanitize(parts, MAX_TITLE_BYTES));
    }

    fn notify(&mut self, kind: NotificationKind, title: &[&[u8]], body: &[&[u8]]) {
        let Some(sequence) = self
            .presentation
            .notification_sequence
            .checked_add(NEXT_NOTIFICATION)
        else {
            return;
        };
        self.presentation.notification_sequence = sequence;
        self.presentation.notification = Some(TerminalNotification {
            kind,
            title: text::sanitize(title, MAX_NOTIFICATION_TITLE_BYTES),
            body: text::sanitize(body, MAX_NOTIFICATION_BODY_BYTES),
        });
    }
}

impl vt100::Callbacks for MetadataCallbacks {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title(&[title]);
    }

    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.notify(NotificationKind::Bell, &[], &[]);
    }

    fn visual_bell(&mut self, _: &mut vt100::Screen) {
        self.notify(NotificationKind::Bell, &[], &[]);
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        match params {
            [OSC_TITLE_AND_ICON | OSC_TITLE, parts @ ..] => self.title(parts),
            [OSC_NOTIFICATION, WINDOWS_PROGRESS, _, ..] => {}
            [OSC_NOTIFICATION, body @ ..] if !body.is_empty() => {
                self.notify(NotificationKind::Desktop, &[], body);
            }
            [OSC_URXVT, URXVT_NOTIFY, title, body @ ..] => {
                self.notify(NotificationKind::Desktop, &[*title], body);
            }
            _ => {}
        }
    }
}
