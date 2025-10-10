use crate::errors::Error;
use crate::i18n::{I18N, LangKey, Language};
use crate::state::Event::SetLanguage;
use crate::state::Message;
use crate::ui::widgets::popups::PopupUi;
use crate::utils::AppSettings;
use egui::Ui;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub struct SettingsPopup {
    language_settings: LanguageSettings,
    theme_settings: ThemeSettings,
}

struct LanguageSettings {
    selected_idx: usize,
    items: Vec<Language>,
}

struct ThemeSettings {
    selected_idx: usize,
    items: Vec<&'static str>,
}

impl Default for SettingsPopup {
    fn default() -> Self {
        Self {
            language_settings: LanguageSettings {
                selected_idx: 0,
                items: Language::vector(),
            },
            theme_settings: ThemeSettings {
                selected_idx: 0,
                items: vec!["system", "light", "dark"],
            },
        }
    }
}

impl PopupUi for SettingsPopup {
    fn ui(
        &mut self,
        ui: &mut Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    ) {
        ui.set_min_width(480.0);
        ui.set_min_height(360.0);
        ui.vertical(|ui| {
            ui.label(i18n.get(LangKey::Language));
            egui::ComboBox::from_label(i18n.get(LangKey::SelectLanguage))
                .selected_text(
                    self.language_settings.items[self.language_settings.selected_idx].to_string(),
                )
                .show_ui(ui, |ui| {
                    for (i, option) in self.language_settings.items.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.language_settings.selected_idx,
                            i,
                            option.to_string(),
                        );
                    }
                });
            ui.separator();
            ui.label("Theme");
            egui::ComboBox::from_label("Select Theme")
                .selected_text(
                    self.theme_settings.items[self.theme_settings.selected_idx].to_string(),
                )
                .show_ui(ui, |ui| {
                    for (i, option) in self.theme_settings.items.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.theme_settings.selected_idx,
                            i,
                            option.to_string(),
                        );
                    }
                });
            ui.separator();
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    egui::Sides::new().show(
                        ui,
                        |_| {},
                        |ui| {
                            if ui.button(i18n.get(LangKey::Apply)).clicked() {
                                self.update_language(sender.clone());
                                self.update_theme(sender.clone(), ui.ctx());
                            }
                            if ui.button(i18n.get(LangKey::Cancel)).clicked() {
                                *open = false;
                            }
                            if ui.button(i18n.get(LangKey::Ok)).clicked() {
                                self.update_language(sender.clone());
                                self.update_theme(sender.clone(), ui.ctx());
                                *open = false;
                            }
                        },
                    );
                });
            });
        });
    }
}

impl SettingsPopup {
    pub fn new(current_language: Language) -> Self {
        let selected_idx = Language::vector()
            .iter()
            .position(|&x| x == current_language)
            .unwrap_or(0);

        let app_settings = AppSettings::new_from_file();
        let current_theme = app_settings.get_theme();
        let theme_items = vec!["system", "light", "dark"];
        let theme_selected_idx = theme_items
            .iter()
            .position(|&x| x == current_theme)
            .unwrap_or(0);

        Self {
            language_settings: LanguageSettings {
                selected_idx,
                items: Language::vector(),
            },
            theme_settings: ThemeSettings {
                selected_idx: theme_selected_idx,
                items: theme_items,
            },
        }
    }
    fn update_language(&mut self, sender: Arc<Sender<Message>>) {
        let app_settings = AppSettings::new_from_file();
        if let Err(e) = app_settings.set_language(&self.selected_language()) {
            e.show_error_dialog(sender.clone());
        };

        if let Err(e) = app_settings.save_to_file() {
            e.show_error_dialog(sender.clone());
        };
        let sender = sender.clone();
        sender
            .send(Message::Event(Arc::new(SetLanguage(
                self.selected_language(),
            ))))
            .unwrap_or_else(|e| {
                Error::from(e).show_error_dialog(sender);
            });
    }

    fn selected_language(&self) -> Language {
        self.language_settings.items[self.language_settings.selected_idx]
    }

    fn update_theme(&mut self, sender: Arc<Sender<Message>>, ctx: &egui::Context) {
        let app_settings = AppSettings::new_from_file();
        let theme = self.selected_theme();

        if let Err(e) = app_settings.set_theme(theme) {
            e.show_error_dialog(sender.clone());
        };

        if let Err(e) = app_settings.save_to_file() {
            e.show_error_dialog(sender.clone());
        };

        match theme {
            "light" => ctx.set_theme(egui::Theme::Light),
            "dark" => ctx.set_theme(egui::Theme::Dark),
            _ => {
                if ctx
                    .native_pixels_per_point()
                    .is_some_and(|_| ctx.style().visuals.dark_mode)
                {
                    ctx.set_theme(egui::Theme::Dark);
                } else {
                    ctx.set_theme(egui::Theme::Light);
                }
            }
        }
    }

    fn selected_theme(&self) -> &str {
        self.theme_settings.items[self.theme_settings.selected_idx]
    }
}
