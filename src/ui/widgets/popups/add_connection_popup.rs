use crate::errors::Error;
use crate::i18n::{I18N, LangKey};
use crate::state::Event::{AddServer, DeleteServer, ShowInfo};
use crate::state::Message::Event;
use crate::state::{Info, Message};
use crate::ui::widgets::popups::PopupUi;
use crate::utils::{ValkeyClient, ValkeyUrl, ValkeyUrlBuilder};
use egui::Ui;
use egui::mutex::RwLock;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

pub struct AddConnectionPopup {
    connection_string: String,
    alias: String,
    old_alias: Option<String>,
    host: String,
    port: String,
    username: String,
    password: String,
    database_index: String,
    show_password: (bool, bool),
    connection_string_focus: bool,
    connected: Arc<RwLock<bool>>,
}
impl Default for AddConnectionPopup {
    fn default() -> Self {
        Self {
            connection_string: "".to_string(),
            alias: "".to_string(),
            old_alias: None,
            host: "".to_string(),
            port: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            database_index: "".to_string(),
            show_password: (false, true),
            connection_string_focus: false,
            connected: Default::default(),
        }
    }
}

impl PopupUi for AddConnectionPopup {
    fn ui(
        &mut self,
        ui: &mut Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    ) {
        self.parse_valkey_url();

        if !self.port.is_empty() {
            match self.port.parse::<i32>() {
                Ok(n) => {
                    if n > 65535 {
                        self.port = 65535.to_string();
                    } else if n < 0 {
                        self.port = 0.to_string();
                    } else {
                        self.port = n.to_string();
                    }
                }
                Err(_) => {
                    self.port = "0".to_string();
                }
            };
        }

        if !self.database_index.is_empty() {
            match self.database_index.parse::<i64>() {
                Ok(n) => {
                    if n > 4294967295 {
                        self.database_index = 4294967295i64.to_string();
                    } else if n < 0 {
                        self.database_index = 0.to_string();
                    } else {
                        self.database_index = n.to_string();
                    }
                }
                Err(_) => {
                    self.database_index = "0".to_string();
                }
            };
        }

        ui.label(i18n.get(LangKey::ConnectionString));
        let connection_string_response = ui.add(
            egui::TextEdit::singleline(&mut self.connection_string)
                .desired_width(ui.available_width())
                .hint_text("valkey://127.0.0.1:6379"),
        );
        self.connection_string_focus = connection_string_response.has_focus();
        if !self.show_password.0 && connection_string_response.lost_focus() {
            self.show_password.1 = false;
        }
        if ui
            .checkbox(&mut self.show_password.0, i18n.get(LangKey::ShowPassword))
            .clicked()
        {
            self.handle_show_password();
        };
        ui.separator();
        ui.label(format!("{}*", i18n.get(LangKey::DatabaseAlias)));
        ui.add(
            egui::TextEdit::singleline(&mut self.alias)
                .desired_width(ui.available_width())
                .hint_text(i18n.get(LangKey::ValkeyDatabase)),
        );
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("{}*", i18n.get(LangKey::Host)));
                ui.add(
                    egui::TextEdit::singleline(&mut self.host)
                        .desired_width(ui.available_width() * 0.7)
                        .hint_text("127.0.0.1"),
                );
            });
            ui.vertical(|ui| {
                ui.label(format!("{}*", i18n.get(LangKey::Port)));
                ui.add(
                    egui::TextEdit::singleline(&mut self.port)
                        .desired_width(ui.available_width())
                        .hint_text("6379"),
                );
            })
        });
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(i18n.get(LangKey::Username));
                ui.add(
                    egui::TextEdit::singleline(&mut self.username)
                        .desired_width(ui.available_width() * 0.5)
                        .hint_text("Admin"),
                );
            });
            ui.vertical(|ui| {
                ui.label(i18n.get(LangKey::Password));
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(ui.available_width())
                            .hint_text("Pa$$w0rd"),
                    )
                    .has_focus()
                {
                    self.handle_show_password();
                }
            });
        });
        ui.vertical(|ui| {
            ui.label(i18n.get(LangKey::DatabaseIndex));
            ui.add(
                egui::TextEdit::singleline(&mut self.database_index)
                    .desired_width(ui.available_width() * 0.5)
                    .hint_text("0"),
            );
        });
        ui.separator();
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    if ui.button(i18n.get(LangKey::TestConnection)).clicked() {
                        self.connect(sender.clone(), i18n.clone(), true);
                    };
                    if *self.connected.read() {
                        ui.label(i18n.get(LangKey::ConnectionSuccess));
                    }
                })
            },
            |ui| {
                ui.horizontal(|ui| {
                    if ui.button(i18n.get(LangKey::Save)).clicked() {
                        self.connect(sender.clone(), i18n.clone(), false);
                        *open = false;
                    };
                    if ui.button(i18n.get(LangKey::Cancel)).clicked() {
                        *open = false;
                    }
                })
            },
        );
    }
}

impl AddConnectionPopup {
    fn connect(&self, sender: Arc<Sender<Message>>, i18n: Arc<I18N>, test: bool) {
        if self.alias.is_empty() {
            Self::info_dialog(
                sender.clone(),
                i18n.clone(),
                &i18n.get(LangKey::AliasRequiredField),
            );
            return;
        }

        if self.host.is_empty() {
            Self::info_dialog(
                sender.clone(),
                i18n.clone(),
                &i18n.get(LangKey::HostRequiredField),
            );
            return;
        }

        let connection_string = match self.get_url_with_cleartext_pw() {
            Ok(url_with_pw) => url_with_pw,
            Err(e) => {
                e.show_error_dialog(sender.clone());
                return;
            }
        };
        let alias = self.alias.clone();
        let old_alias = self.old_alias.clone();
        let connected = self.connected.clone();
        thread::spawn(move || {
            match ValkeyClient::new(
                None.into(),
                Arc::new(connection_string),
                sender.clone(),
                i18n.clone(),
            ) {
                Ok(vc) => {
                    let mut connected = connected.write();
                    *connected = true;
                    if !test
                        && let Some(old_alias) = old_alias
                        && let Err(e) = sender
                            .send(Event(Arc::new(DeleteServer(old_alias))))
                            .map_err(Error::from)
                    {
                        e.show_error_dialog(sender.clone());
                        return;
                    }

                    if let Err(e) = sender
                        .send(Event(Arc::new(AddServer(alias, vc.server_url()))))
                        .map_err(Error::from)
                    {
                        e.show_error_dialog(sender);
                    }
                }
                Err(e) => {
                    let mut connected = connected.write();
                    *connected = false;
                    e.show_error_dialog(sender);
                }
            }
        });
    }

    pub fn new(alias: &str, connection_string: &str) -> Self {
        if let Ok(valkey_url) = ValkeyUrl::parse_valkey_url(None, connection_string) {
            Self {
                connection_string: connection_string.to_string(),
                alias: alias.to_string(),
                old_alias: Some(alias.to_string()),
                host: valkey_url.host().to_string(),
                port: valkey_url.port().to_string(),
                username: valkey_url.username().unwrap_or("").to_string(),
                password: valkey_url.password().unwrap_or("").to_string(),
                database_index: valkey_url
                    .db()
                    .map(|dbi| dbi.to_string())
                    .unwrap_or_default(),
                show_password: (false, false),
                connection_string_focus: false,
                connected: Default::default(),
            }
        } else {
            Self {
                connection_string: connection_string.to_string(),
                alias: alias.to_string(),
                old_alias: Some(alias.to_string()),
                host: "".to_string(),
                port: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                database_index: "".to_string(),
                show_password: (false, false),
                connection_string_focus: false,
                connected: Default::default(),
            }
        }
    }

    fn info_dialog(sender: Arc<Sender<Message>>, i18n: Arc<I18N>, message: &str) {
        if let Err(e) = sender
            .send(Event(Arc::from(ShowInfo(Info {
                title: i18n.get(LangKey::EmptyRequiredFields),
                message: message.to_string(),
                callback: Some(|| {}),
            }))))
            .map_err(Error::from)
        {
            e.show_error_dialog(sender.clone());
        }
    }

    fn get_url_with_cleartext_pw(&self) -> Result<String, Box<Error>> {
        let valkey_url_builder = ValkeyUrlBuilder::from(self.connection_string.clone());
        Ok(valkey_url_builder
            .password(self.password.clone())
            .build()?
            .connection_string())
    }

    fn parse_valkey_url(&mut self) {
        if !self.connection_string_focus && !self.host.is_empty() && !self.port.is_empty() {
            let mut valkey_url = ValkeyUrlBuilder::new();
            valkey_url = valkey_url.connection_name(self.alias.clone());
            valkey_url = valkey_url.host(self.host.clone());
            valkey_url = valkey_url.port(self.port.parse().unwrap_or(0));
            if !self.username.is_empty() {
                valkey_url = valkey_url.username(self.username.clone())
            }
            if !self.password.is_empty() {
                if self.show_password.0 || self.show_password.1 {
                    valkey_url = valkey_url.password(self.password.clone())
                } else {
                    valkey_url = valkey_url.password("*".repeat(self.password.len()))
                }
            }
            if !self.database_index.is_empty() {
                valkey_url = valkey_url.db(self.database_index.parse().unwrap_or(0))
            }
            match valkey_url.build() {
                Ok(valkey_url) => {
                    self.connection_string = valkey_url.connection_string();
                }
                Err(e) => {
                    eprintln!("Cannot convert valkey url:{}", e);
                }
            };
        } else if let Ok(valkey_url) = ValkeyUrl::parse_valkey_url(None, &self.connection_string) {
            self.host = valkey_url.host().to_string();
            self.port = valkey_url.port().to_string();
            self.username = valkey_url.username().unwrap_or("").to_string();
            if self.show_password.0 || self.show_password.1 {
                self.password = valkey_url.password().unwrap_or("").to_string();
            }
            self.database_index = match valkey_url.db() {
                Some(db_idx) => db_idx.to_string(),
                None => "".to_string(),
            }
        }
    }

    fn handle_show_password(&mut self) {
        if !self.show_password.0 && self.show_password.1 {
            self.show_password.1 = false;
        } else if self.show_password.0 && !self.show_password.1 {
            self.show_password.1 = true;
        }
    }
}
