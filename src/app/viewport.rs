use crate::render::Layout;

const SIDEBAR_MIN: u16 = 10;
const TERM_MIN_COLS: u16 = 20;

pub(super) struct Viewport {
    cols: u16,
    rows: u16,
    sidebar_visible: bool,
    sidebar_width: u16,
    dragging_sidebar: bool,
}

impl Viewport {
    pub(super) fn sidebar_visible(&self) -> bool {
        self.sidebar_visible
    }
    pub(super) fn is_dragging_sidebar(&self) -> bool {
        self.dragging_sidebar
    }
    pub(super) fn begin_sidebar_drag(&mut self) {
        self.dragging_sidebar = true;
    }
    pub(super) fn end_sidebar_drag(&mut self) {
        self.dragging_sidebar = false;
    }

    pub(super) fn configured(
        cols: u16,
        rows: u16,
        sidebar_visible: bool,
        sidebar_width: u16,
    ) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            sidebar_visible,
            sidebar_width: sidebar_width.max(SIDEBAR_MIN),
            dragging_sidebar: false,
        }
    }

    pub(super) fn layout(&self) -> Layout {
        Layout::new(
            self.cols,
            self.rows,
            self.sidebar_visible,
            self.sidebar_width,
        )
    }

    pub(super) fn resize(&mut self, cols: u16, rows: u16) -> bool {
        let size = (cols.max(1), rows.max(1));
        if size == (self.cols, self.rows) {
            return false;
        }
        (self.cols, self.rows) = size;
        true
    }

    pub(super) fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub(super) fn show_wider(&mut self) {
        self.sidebar_visible = true;
        self.sidebar_width = self
            .sidebar_width
            .saturating_add(2)
            .clamp(SIDEBAR_MIN, self.sidebar_max());
    }

    pub(super) fn adjust_sidebar(&mut self, delta: i16) -> bool {
        let next = (self.sidebar_width as i16 + delta)
            .clamp(SIDEBAR_MIN as i16, self.sidebar_max() as i16) as u16;
        let changed = next != self.sidebar_width;
        self.sidebar_width = next;
        changed
    }

    pub(super) fn set_sidebar_width(&mut self, width: u16) -> bool {
        let width = width.clamp(SIDEBAR_MIN, self.sidebar_max());
        let changed = width != self.sidebar_width || !self.sidebar_visible;
        self.sidebar_width = width;
        self.sidebar_visible = true;
        changed
    }

    fn sidebar_max(&self) -> u16 {
        self.cols.saturating_sub(TERM_MIN_COLS).max(SIDEBAR_MIN)
    }
}
