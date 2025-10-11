use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::{AppState, Message};
use crate::ui::Component;
use crate::ui::widgets::PopupType;
use crate::utils::ValkeyClient;
use egui::Context;
use std::process::Command;
use std::sync::Arc;
use std::{env, thread};

#[derive(Default)]
pub struct MenuBar;

impl Component for MenuBar {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        let self_path = env::current_exe()?;
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(state.i18n().get(LangKey::Window), |ui| {
                    if ui.button(state.i18n().get(LangKey::NewWindow)).clicked() {
                        let mut child = Command::new("setsid")
                            .arg(self_path)
                            .spawn()
                            .unwrap_or_else(|_| {
                                panic!("{}", state.i18n().get(LangKey::FailedSpawnDetachedInstance))
                            });
                        thread::spawn(move || {
                            let _ = child.wait();
                        });
                        ui.close();
                    }
                    if ui
                        .button(if state.ui_panels.left_side_bar_open {
                            state.i18n().get(LangKey::HideSidebar)
                        } else {
                            state.i18n().get(LangKey::ShowSidebar)
                        })
                        .clicked()
                    {
                        state.set_state(Message::ToggleSidebar);
                        ui.close();
                    }
                    if ui.button(state.i18n().get(LangKey::Quit)).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });
                ui.menu_button(state.i18n().get(LangKey::Edit), |ui| {
                    if ui.button(state.i18n().get(LangKey::Settings)).clicked() {
                        state
                            .get_sender()
                            .send(Message::OpenPopup(PopupType::Settings(Box::default())))
                            .unwrap_or_else(|e| {
                                Error::from(e).show_error_dialog(state.get_sender());
                            });
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        state.show_about = true;
                        ui.close();
                    }
                });
            });
        });
        Ok(())
    }

    fn refresh(&mut self, _: &Arc<ValkeyClient>) {}
}
