use crate::errors::Error;
use crate::i18n::{I18N, LangKey};
use crate::state::Message;
use crate::ui;
use crate::ui::widgets::popups::{AddConnectionPopup, AddKey, EditKey, PopupUi, SettingsPopup};
use crate::utils::random_string;
use egui::{Context, Vec2b};
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub struct Popup {
    pub id: String,
    pub title: String,
    pub popup_type: PopupType,
    pub resizable: bool,
    pub open: bool,
}

impl Default for Popup {
    fn default() -> Self {
        Self {
            id: random_string(32).unwrap_or("".into()),
            title: "Default".to_string(),
            popup_type: PopupType::default(),
            resizable: false,
            open: false,
        }
    }
}

#[derive(Default)]
pub enum PopupType {
    #[default]
    Undef,
    AddConnection(Box<AddConnectionPopup>),
    AddKey(Box<AddKey>),
    EditKey(Box<EditKey>),
    Settings(Box<SettingsPopup>),
}

impl PopupUi for PopupType {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    ) {
        match self {
            PopupType::Undef => {
                ui.label(i18n.get(LangKey::Undefined));
                ui.separator();
                ui.label(i18n.get(LangKey::NothingToDisplay));
            }
            PopupType::AddConnection(popup) => {
                popup.ui(ui, sender, i18n, open);
            }
            PopupType::Settings(popup) => {
                popup.ui(ui, sender, i18n, open);
            }
            PopupType::AddKey(popup) => {
                popup.ui(ui, sender, i18n, open);
            }
            PopupType::EditKey(popup) => {
                popup.ui(ui, sender, i18n, open);
            }
        }
    }
}

impl Popup {
    pub fn new(popup_type: PopupType, resizable: bool, i18n: &I18N) -> Self {
        Self {
            id: "popup".into(),
            title: match &popup_type {
                PopupType::Undef => i18n.get(LangKey::Settings),
                PopupType::AddConnection(_) => i18n.get(LangKey::AddConnection),
                PopupType::Settings(_) => i18n.get(LangKey::Settings),
                PopupType::AddKey(_) => i18n.get(LangKey::NewKey),
                PopupType::EditKey(edit_key) => {
                    format!("{} - {}", i18n.get(LangKey::EditKey), edit_key.key_name())
                }
            },
            popup_type,
            resizable,
            ..Default::default()
        }
    }
}

impl ui::Widget for Popup {
    fn show(
        &mut self,
        ctx: &Context,
        sender: Arc<Sender<Message>>,
        i18n: Arc<I18N>,
        collapsable: bool,
        resizable: bool,
    ) -> Result<(), Error> {
        let mut should_close = false;
        let display_title = if self.title.len() > 48 {
            format!("{}...", &self.title[..48])
        } else {
            self.title.clone()
        };

        egui::Window::new(display_title)
            .id(egui::Id::new(self.id.clone()))
            .min_width(480.0)
            .min_height(360.0)
            .collapsible(collapsable)
            .resizable(Vec2b {
                x: resizable,
                y: false,
            })
            .open(&mut self.open)
            .show(ctx, |ui| match &mut self.popup_type {
                PopupType::Undef => {
                    ui.label(i18n.get(LangKey::Undefined));
                    ui.separator();
                    ui.label(i18n.get(LangKey::NothingToDisplay));
                }
                popup => {
                    let mut popup_open = true;
                    popup.ui(ui, &sender, &i18n, &mut popup_open);
                    if !popup_open {
                        should_close = true;
                    }
                }
            });

        if should_close {
            self.open = false;
        }

        Ok(())
    }
}
