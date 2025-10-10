use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::Event::SetMainWindow;
use crate::state::{AppState, MainWindow, Message};
use crate::ui::Component;
use crate::utils::ValkeyClient;
use egui::Context;
use std::sync::Arc;

#[derive(Default)]
pub struct LeftSideBar;

impl Component for LeftSideBar {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        let current_window = *state.ui_panels.current_window.read()?;
        egui::SidePanel::left("left_side_bar")
            .min_width(80.0)
            .max_width(240.0)
            .default_width(120.0)
            .show(ctx, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 0.0],
                        egui::Button::new(state.i18n().get(LangKey::Connections))
                            .selected(matches!(current_window, Some(MainWindow::Connection))),
                    )
                    .clicked()
                {
                    self.set_window_state(state, MainWindow::Connection)
                        .unwrap_or_else(|e| {
                            e.show_error_dialog(state.get_sender());
                        });
                };
                if state.valkey_client.is_some() {
                    ui.separator();
                    if ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new(state.i18n().get(LangKey::Browser))
                                .selected(matches!(current_window, Some(MainWindow::Browser))),
                        )
                        .clicked()
                    {
                        self.set_window_state(state, MainWindow::Browser)
                            .unwrap_or_else(|e| {
                                e.show_error_dialog(state.get_sender());
                            });
                    };
                    if ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new(state.i18n().get(LangKey::Workbench))
                                .selected(matches!(current_window, Some(MainWindow::Workbench))),
                        )
                        .clicked()
                    {
                        self.set_window_state(state, MainWindow::Workbench)
                            .unwrap_or_else(|e| {
                                e.show_error_dialog(state.get_sender());
                            });
                    };
                    if ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new(state.i18n().get(LangKey::Insights))
                                .selected(matches!(current_window, Some(MainWindow::Insights))),
                        )
                        .clicked()
                    {
                        self.set_window_state(state, MainWindow::Insights)
                            .unwrap_or_else(|e| {
                                e.show_error_dialog(state.get_sender());
                            });
                    };
                    if ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new(state.i18n().get(LangKey::Documentation)).selected(
                                matches!(current_window, Some(MainWindow::Documentation)),
                            ),
                        )
                        .clicked()
                    {
                        self.set_window_state(state, MainWindow::Documentation)
                            .unwrap_or_else(|e| {
                                e.show_error_dialog(state.get_sender());
                            });
                    };
                }
            });
        Ok(())
    }

    fn refresh(&mut self, _: &Arc<ValkeyClient>) {}
}
impl LeftSideBar {
    fn set_window_state(&self, state: &AppState, event: MainWindow) -> Result<(), Error> {
        let mut current_window = state.ui_panels.current_window.write()?;
        *current_window = Some(event);
        state
            .get_sender()
            .send(Message::Event(Arc::from(SetMainWindow(event))))?;
        Ok(())
    }
}
