#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::egui::ViewportBuilder;
use crate::egui::vec2;
use eframe::egui;
use valkey_insight::state::AppState;

use eframe::HardwareAcceleration;
use valkey_insight::ui::components::UIComponents;
use valkey_insight::ui::widgets::ErrorModal;
use valkey_insight::ui::{Component, Widget};

struct App {
    ui_components: UIComponents,
    state: AppState,
    frame_count: u16,
}

impl App {
    fn new() -> Self {
        Self {
            ui_components: UIComponents::default(),
            state: AppState::default(),
            frame_count: 0,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.frame_count == 0 {
            let settings = self.state.get_settings();
            let theme = settings.get_theme();
            match theme.as_str() {
                "light" => ctx.set_theme(egui::Theme::Light),
                "dark" => ctx.set_theme(egui::Theme::Dark),
                _ => { /* No theme selected (System)*/ }
            }
        }

        let sender = self.state.get_sender();
        self.state.get_state(&mut self.ui_components);

        if let Err(e) = self.ui_components.menu_bar.show(ctx, &mut self.state) {
            self.state.error = ErrorModal::from(e);
        };

        if self.state.ui_panels.left_side_bar_open
            && let Err(e) = self.ui_components.left_side_bar.show(ctx, &mut self.state)
        {
            self.state.error = ErrorModal::from(e);
        };

        if let Err(e) = self.ui_components.current_window.show(ctx, &mut self.state) {
            self.state.error = ErrorModal::from(e);
        }

        if self.state.error.open {
            egui::Modal::new(egui::Id::new("critical_error")).show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(&self.state.error.title);
                });
                ui.separator();
                ui.add_space(8.0);
                ui.label(&self.state.error.error_message);
                ui.add_space(8.0);
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("Ok").clicked() {
                        self.state.error.open = false;
                    }
                });
            });
        }

        if self.state.info.open {
            egui::Modal::new(egui::Id::new("info")).show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(&self.state.info.title);
                });
                ui.separator();
                ui.add_space(8.0);
                ui.label(&self.state.info.message);
                ui.add_space(8.0);
                if self.state.info.on_close.is_some() {
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button("Ok").clicked() {
                            self.state.info.open = false;
                        }
                    });
                }
            });
        }

        let i18n = self.state.i18n();
        self.state.popups.retain_mut(|popup| {
            if popup.open {
                popup
                    .show(ctx, sender.clone(), i18n.clone(), true, popup.resizable)
                    .unwrap_or_else(|e| {
                        self.state.error = ErrorModal::from(e);
                    })
            }
            popup.open
        });

        if self.state.settings_popup.open {
            self.state
                .settings_popup
                .show(
                    ctx,
                    sender.clone(),
                    i18n.clone(),
                    false,
                    self.state.settings_popup.resizable,
                )
                .unwrap_or_else(|e| {
                    self.state.error = ErrorModal::from(e);
                });
        }

        if self.frame_count >= 1024 {
            self.state
                .get_settings()
                .load_from_file()
                .unwrap_or_else(|e| {
                    e.show_error_dialog(sender.clone());
                });
            self.frame_count = 0;
        } else {
            self.frame_count += 1;
        }

        ctx.request_repaint();
    }
}

fn main() {
    let viewport = ViewportBuilder::default()
        .with_min_inner_size(vec2(800.0, 600.0))
        .with_inner_size(vec2(1366.0, 768.0));

    let native_options = eframe::NativeOptions {
        viewport,
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        hardware_acceleration: HardwareAcceleration::Preferred,
        renderer: Default::default(),
        run_and_return: false,
        event_loop_builder: None,
        window_builder: None,
        shader_version: None,
        centered: true,
        persist_window: false,
        persistence_path: None,
        dithering: false,
    };

    eframe::run_native(
        "Valkey Insight",
        native_options,
        Box::new(|_| Ok(Box::new(App::new()))),
    )
    .expect("A critical error occurred starting the app.");
}
