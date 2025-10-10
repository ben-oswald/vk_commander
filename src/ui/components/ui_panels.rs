use crate::state::MainWindow;
use std::sync::{Arc, RwLock};

pub struct UIPanels {
    pub left_side_bar_open: bool,
    pub current_window: Arc<RwLock<Option<MainWindow>>>,
}

impl Default for UIPanels {
    fn default() -> Self {
        Self {
            left_side_bar_open: true,
            current_window: Arc::new(RwLock::new(Some(MainWindow::Connection))),
        }
    }
}
