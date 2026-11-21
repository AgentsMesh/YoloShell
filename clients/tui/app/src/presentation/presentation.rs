use crate::Mode;
use ratatui::layout::Rect;
use yolo_presentation_layout::Layout;
use yolo_tui_render::PaneViewport;

pub struct Presentation {
    pub bounds: Rect,
    pub panes: Vec<PaneViewport>,
    pub layout: Option<Layout>,
    pub mode: Mode,
    pub hub_button: Option<Rect>,
    pub hub_view: Option<yolo_tui_hub_manager::View>,
    pub(crate) help: Option<crate::help::Viewport>,
    pub zoomed: bool,
    pub inventory: Option<yolo_tui_inventory::InventoryViewport>,
    pub(crate) tab_strip: Vec<crate::tabs::strip::Tab>,
    pub(crate) tab_ids: Vec<uuid::Uuid>,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            bounds: Rect::default(),
            panes: Vec::new(),
            layout: None,
            mode: Mode::Inventory,
            hub_button: None,
            hub_view: None,
            help: None,
            zoomed: false,
            inventory: None,
            tab_strip: Vec::new(),
            tab_ids: Vec::new(),
        }
    }
}
impl Presentation {
    pub fn current(&self, layout: &Layout, mode: Mode) -> bool {
        self.mode == mode && self.layout.as_ref() == Some(layout)
    }
    pub fn geometry_bounds(&self) -> yolo_presentation_layout::Rect {
        yolo_presentation_layout::Rect {
            x: self.bounds.x,
            y: self.bounds.y,
            width: self.bounds.width,
            height: self.bounds.height,
        }
    }
}
