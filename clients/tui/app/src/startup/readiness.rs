use crate::State;
use ratatui::layout::Rect;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{TerminalSize, TerminalSummary};

impl State {
    /// Geometry for the next focused PTY resize, computed once by each draw.
    pub(crate) fn focused_terminal_size(&self, bounds: Rect) -> Option<TerminalSize> {
        let geometry = self
            .display_layout()
            .geometry(yolo_presentation_layout::Rect {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: bounds.height,
            })
            .ok()?;
        let rect = geometry
            .into_iter()
            .find(|placement| placement.pane == self.tab_layout().focused())?
            .rect;
        let area = Rect::new(rect.x, rect.y, rect.width, rect.height);
        let inner = yolo_tui_render::terminal_inner(area, bounds);
        TerminalSize::new(inner.width, inner.height).ok()
    }

    /// A label must not promise synchronized geometry before run queues resize.
    /// Background terminals retain their actual writer/readiness presentation.
    pub(crate) fn presentation_ready(
        &self,
        address: TerminalAddress,
        focused: Option<TerminalAddress>,
        desired: Option<TerminalSize>,
    ) -> bool {
        self.input_ready(address)
            && (focused != Some(address)
                || desired.is_some_and(|size| self.last_size(address) == Some(size)))
    }

    pub fn input_ready(&self, address: TerminalAddress) -> bool {
        // A resize that has not completed makes the presentation/geometry
        // unready even though an established keyboard stream may continue.
        if self.is_resize_pending(address) || self.input_blocked(address) {
            return false;
        }
        let Some(surface) = self.surfaces.get(&address) else {
            return false;
        };
        self.last_size(address)
            .is_some_and(|size| surface.screen().size() == (size.rows(), size.cols()))
    }

    /// Conditions under which admitted bytes must not be sent to the PTY.
    ///
    /// This is the single gate shared by direct delivery, pregrant flush and
    /// input admission. A planned geometry refresh with an established,
    /// current surface is not a blocker for keyboard input; that is the
    /// `keyboard_ready` allowance, not a second readiness authority.
    pub(crate) fn input_blocked(&self, address: TerminalAddress) -> bool {
        self.is_offline(address)
            || self.is_resync(address)
            || (self.is_geometry_syncing(address)
                && !self.surfaces.get(&address).is_some_and(|surface| {
                    crate::observation_events::current(self, surface.cursor().target)
                }))
    }

    /// A planned geometry refresh preserves the established keyboard stream.
    /// Mouse coordinates and screen queries still require input_ready().
    pub(crate) fn keyboard_ready(&self, address: TerminalAddress) -> bool {
        self.input_ready(address)
            || (self.is_geometry_syncing(address)
                && !self.is_offline(address)
                && self.surfaces.get(&address).is_some_and(|surface| {
                    crate::observation_events::current(self, surface.cursor().target)
                }))
    }

    pub(crate) fn geometry_refresh_active(&self, address: TerminalAddress) -> bool {
        self.is_geometry_syncing(address)
            && (self.resize_in_flight(address).is_some()
                || self.is_resync(address)
                || !self.observer_generation(address).is_some())
    }
    pub(crate) fn detect_resized_observers(&mut self, terminals: &[TerminalSummary]) {
        for session in terminals {
            let fields = session.fields();
            if self.writers.contains_key(&fields.address) {
                continue;
            }
            let Some(runtime) = fields.runtime else {
                continue;
            };
            let Some(size) = runtime.size else {
                continue;
            };
            if self.surfaces.get(&fields.address).is_some_and(|surface| {
                surface.cursor().target == runtime.target
                    && surface.screen().size() != (size.rows(), size.cols())
            }) {
                self.mark_resync(fields.address);
            }
        }
    }
}
