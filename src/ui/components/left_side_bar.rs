use crate::{
    errors::Error,
    i18n::LangKey,
    state::{AppState, Event::SetMainWindow, MainWindow, Message},
    ui::Component,
    utils::ValkeyClient,
};
use std::sync::Arc;

#[derive(Default)]
pub struct LeftSideBar;

impl Component for LeftSideBar {
    fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Result<(), Error> {
        let current_window = *state.ui_panels.current_window.read()?;
        egui::SidePanel::left("left_side_bar")
            .resizable(false)
            .default_width(64.0)
            .show(ctx, |ui| {
                self.sidebar_nav_button(
                    ui,
                    state,
                    "🔌",
                    state.i18n().get(LangKey::Connections),
                    matches!(current_window, Some(MainWindow::Connection)),
                    MainWindow::Connection,
                );

                if state.valkey_client.is_some() {
                    ui.separator();

                    for (icon, key, target) in [
                        ("🔑", LangKey::Browser, MainWindow::Browser),
                        ("⚒", LangKey::Workbench, MainWindow::Workbench),
                        ("📊", LangKey::Insights, MainWindow::Insights),
                        ("📖", LangKey::Documentation, MainWindow::Documentation),
                    ] {
                        let sel = matches!(current_window, Some(ref v) if std::mem::discriminant(v) == std::mem::discriminant(&target));
                        self.sidebar_nav_button(ui, state, icon, state.i18n().get(key), sel, target);
                    }
                }
            });
        Ok(())
    }

    fn refresh(&mut self, _: &Arc<ValkeyClient>) {}
}
impl LeftSideBar {
    fn sidebar_nav_button(
        &self,
        ui: &mut egui::Ui,
        state: &AppState,
        icon: &str,
        label: impl Into<egui::WidgetText>,
        selected: bool,
        target: MainWindow,
    ) {
        if ui
            .scope(|ui| {
                if !selected {
                    let visuals = &mut ui.style_mut().visuals.widgets.inactive;
                    visuals.bg_fill = egui::Color32::TRANSPARENT;
                    visuals.weak_bg_fill = egui::Color32::TRANSPARENT;
                    visuals.bg_stroke = egui::Stroke::NONE;
                }
                ui.add_sized(
                    [48.0, 48.0],
                    egui::Button::new(egui::RichText::new(icon).size(32.0)).selected(selected),
                )
                .on_hover_text(label)
            })
            .inner
            .clicked()
        {
            self.set_window_state(state, target)
                .unwrap_or_else(|e| e.show_error_dialog(state.get_sender()));
        }
    }

    fn set_window_state(&self, state: &AppState, event: MainWindow) -> Result<(), Error> {
        *state.ui_panels.current_window.write()? = Some(event);
        state
            .get_sender()
            .send(Message::Event(Arc::from(SetMainWindow(event))))?;
        Ok(())
    }
}
