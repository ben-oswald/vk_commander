mod browser_window;
mod connections_window;
mod documentation_window;
mod insights_window;
mod left_side_bar;
mod menu_bar;
mod ui_panels;
mod workbench_window;

use crate::ui::Component;
use crate::ui::widgets::Popup;
pub use browser_window::{BrowserWindow, KeyMetadata};
pub use connections_window::ConnectionsWindow;
pub use documentation_window::DocumentationWindow;
pub use insights_window::InsightsWindow;
pub use left_side_bar::LeftSideBar;
pub use menu_bar::MenuBar;
pub use ui_panels::UIPanels;
pub use workbench_window::WorkbenchWindow;

pub struct UIComponents {
    pub popup_window: Popup,
    pub ui_panels: UIPanels,
    pub menu_bar: MenuBar,
    pub left_side_bar: LeftSideBar,
    pub current_window: Box<dyn Component>,
}

impl Default for UIComponents {
    fn default() -> Self {
        Self {
            popup_window: Default::default(),
            ui_panels: Default::default(),
            menu_bar: Default::default(),
            left_side_bar: Default::default(),
            current_window: Box::new(ConnectionsWindow::default()),
        }
    }
}
