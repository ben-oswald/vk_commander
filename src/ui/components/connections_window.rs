use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::Event::{SetConnection, SetMainWindow};
use crate::state::Message::Event;
use crate::state::{AppState, MainWindow, Message};
use crate::ui::Component;
use crate::ui::widgets::{AddConnectionPopup, PopupType};
use crate::utils::{ValkeyClient, ValkeyUrl};
use egui::{Align, Button, Context, Direction, Id, Label, Layout, Modal, TextWrapMode, Ui};
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Default)]
pub struct ConnectionsWindow {
    connection_string: String,
    filter: String,
    sorted_by: RwLock<SortedBy>,
    pending_delete: Option<String>,
}
#[derive(Debug)]
enum SortedBy {
    Name(bool /*reversed*/),
    HostPort(bool /*reversed*/),
    ConnectionType(bool /*reversed*/),
    LastConnect(bool /*reversed*/),
}

macro_rules! toggle_variant {
    ($self:ident, $variant:ident) => {{
        match $self {
            SortedBy::$variant(reversed) => {
                *reversed = !*reversed;
            }
            _ => {
                *$self = SortedBy::$variant(false);
            }
        }
    }};
}

impl SortedBy {
    fn toggle_name(&mut self) {
        toggle_variant!(self, Name);
    }

    fn toggle_host_port(&mut self) {
        toggle_variant!(self, HostPort);
    }

    fn toggle_connection_type(&mut self) {
        toggle_variant!(self, ConnectionType);
    }

    fn toggle_last_connect(&mut self) {
        toggle_variant!(self, LastConnect);
    }
}

impl Default for SortedBy {
    fn default() -> Self {
        SortedBy::Name(false)
    }
}

impl Component for ConnectionsWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        egui::CentralPanel::default().show(ctx, |ui| -> Result<(), Error> {
            let servers = state.get_settings().get_servers()?;
            ui.heading(state.i18n().get(LangKey::QuickConnect));
            ui.add_space(10.0);
            ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                ui.add_sized(
                    [ui.available_width() - 120.0, 0.0],
                    egui::TextEdit::singleline(&mut self.connection_string)
                        .hint_text("valkey://127.0.0.1:6379"),
                );

                if ui
                    .add_sized(
                        [100.0, 0.0],
                        Button::new(state.i18n().get(LangKey::Connect)),
                    )
                    .clicked()
                {
                    let connection_string = if self.connection_string.is_empty() {
                        "valkey://127.0.0.1:6379".to_string()
                    } else {
                        self.connection_string.clone()
                    };

                    Self::connect_to_valkey(None, connection_string, state);
                }
            });
            ui.add_space(10.0);
            ui.separator();
            egui::Sides::new().show(
                ui,
                |ui| {
                    if ui
                        .button(state.i18n().get(LangKey::AddConnection))
                        .clicked()
                    {
                        let sender = state.get_sender();
                        sender
                            .send(Message::OpenPopup(PopupType::AddConnection(Box::default())))
                            .unwrap_or_else(|e| Error::from(e).show_error_dialog(sender.clone()));
                    };
                },
                |ui| {
                    if !servers.is_empty() {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.filter)
                                .desired_width(180.0)
                                .hint_text(state.i18n().get(LangKey::SearchConnections)),
                        );
                    }
                },
            );
            ui.separator();

            let sender = state.get_sender();
            self.connection_table(ui, state, &sender)?;
            Ok(())
        });

        if let Some(server_alias) = &self.pending_delete.clone() {
            Modal::new(Id::new("delete_connection")).show(ctx, |ui| {
                ui.set_width(280.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.add(
                                Label::new(
                                    egui::RichText::new(format!(
                                        "{} {}?",
                                        state.i18n().get(LangKey::Delete),
                                        server_alias
                                    ))
                                    .heading(),
                                )
                                .truncate(),
                            );
                        });
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(format!("{}?", state.i18n().get(LangKey::AreYouSure)));
                        });
                    });
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button(state.i18n().get(LangKey::No)).clicked() {
                        self.pending_delete = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(state.i18n().get(LangKey::Yes)).clicked() {
                            let settings = state.get_settings();
                            settings.delete_server(server_alias).unwrap_or_else(|e| {
                                e.show_error_dialog(state.get_sender());
                            });
                            self.pending_delete = None;
                        }
                    });
                });
            });
        }

        Ok(())
    }

    fn refresh(&mut self, _: &Arc<ValkeyClient>) {}
}

impl ConnectionsWindow {
    fn connection_table(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        sender: &Arc<std::sync::mpsc::Sender<Message>>,
    ) -> Result<(), Error> {
        let settings = state.get_settings();
        ui.allocate_ui(ui.available_size(), |ui| {
            let servers = settings.get_servers().unwrap_or_else(|e| {
                e.show_error_dialog(sender.clone());
                HashMap::new()
            });
            if servers.is_empty() {
                ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                    ui.label(state.i18n().get(LangKey::NoConnections));
                });
            } else {
                self.build_connection_table(ui, state).unwrap_or_else(|e| {
                    e.show_error_dialog(sender.clone());
                });
            }
        });
        Ok(())
    }

    fn build_connection_table(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
    ) -> Result<(), Box<Error>> {
        let mut sorted_by = self.sorted_by.try_write()?;
        let settings = state.get_settings();
        let servers = settings.get_servers()?;
        let available_height = ui.available_height();
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .sense(egui::Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto())
            .column(Column::auto().at_least(40.0).clip(true).resizable(true))
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .strong(state.i18n().get(LangKey::DatabaseAlias))
                            .clicked()
                        {
                            sorted_by.toggle_name();
                        }
                        if let SortedBy::Name(reversed) = *sorted_by {
                            ui.label(if reversed { "⬆" } else { "⬇" });
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui.strong("Host:Port").clicked() {
                            sorted_by.toggle_host_port();
                        }
                        if let SortedBy::HostPort(reversed) = *sorted_by {
                            ui.label(if reversed { "⬆" } else { "⬇" });
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .strong(state.i18n().get(LangKey::ConnectionType))
                            .clicked()
                        {
                            sorted_by.toggle_connection_type();
                        }
                        if let SortedBy::ConnectionType(reversed) = *sorted_by {
                            ui.label(if reversed { "⬆" } else { "⬇" });
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .strong(state.i18n().get(LangKey::LastConnection))
                            .clicked()
                        {
                            sorted_by.toggle_last_connect();
                        }
                        if let SortedBy::LastConnect(reversed) = *sorted_by {
                            ui.label(if reversed { "⬆" } else { "⬇" });
                        }
                    });
                });
            })
            .body(|mut body| {
                for server in servers {
                    let server_info = ValkeyUrl::from(&server.1);
                    let row_height = 21.0;
                    body.row(row_height, |mut row| {
                        row.col(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                            ui.add(Label::new(&server.0).selectable(false));
                        });
                        row.col(|ui| {
                            ui.add(
                                Label::new(format!(
                                    "{}:{}",
                                    server_info.host(),
                                    server_info.port()
                                ))
                                .selectable(false),
                            );
                        });
                        row.col(|ui| {
                            let connection_type = server_info.connection_type().unwrap_or("-");
                            ui.add(Label::new(connection_type).selectable(false));
                        });
                        row.col(|ui| {
                            let last_connection = server_info.last_connection().unwrap_or("-");
                            ui.add(Label::new(last_connection).selectable(false));
                        });

                        if row.response().clicked() {
                            Self::connect_to_valkey(
                                Some(server.0.clone()),
                                server_info.to_string(),
                                state,
                            );
                        }

                        row.response().context_menu(|ui| {
                            if ui
                                .add(Button::new(state.i18n().get(LangKey::Edit)))
                                .clicked()
                            {
                                let sender = state.get_sender();
                                sender
                                    .send(Message::OpenPopup(PopupType::AddConnection(Box::from(
                                        AddConnectionPopup::new(&server.0, &server.1),
                                    ))))
                                    .unwrap_or_else(|e| {
                                        Error::from(e).show_error_dialog(sender.clone())
                                    });
                                ui.close();
                            };
                            if ui
                                .add(Button::new(state.i18n().get(LangKey::Delete)))
                                .clicked()
                            {
                                self.pending_delete = Some(server.0.clone());
                                ui.close();
                            };
                        });
                    });
                }
            });
        Ok(())
    }
    fn connect_to_valkey(alias: Option<String>, url: String, state: &mut AppState) {
        let sender = state.get_sender();
        let i18n = state.i18n();
        let settings = state.get_settings();

        thread::spawn(move || {
            match ValkeyClient::new(
                Arc::from(alias.clone()),
                Arc::from(url.clone()),
                sender.clone(),
                i18n,
            ) {
                Ok(vc) => {
                    if let Some(alias_str) = &alias {
                        let server_type = vc.server_type();
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let timestamp = format!("{}", now);

                        if let Ok(valkey_url) = ValkeyUrl::parse_valkey_url(Some(alias_str), &url) {
                            let connection_string = format!(
                                "{}|type:{}|last:{}",
                                valkey_url.connection_string(),
                                server_type,
                                timestamp
                            );
                            let _ = settings.update_server(alias_str, &connection_string);
                        }
                    }

                    if let Err(e) =
                        sender.send(Event(Arc::from(SetMainWindow(MainWindow::Browser))))
                    {
                        Error::from(e).show_error_dialog(sender.clone());
                        return;
                    }

                    if let Err(e) = sender.send(Event(Arc::from(SetConnection(Arc::from(vc))))) {
                        Error::from(e).show_error_dialog(sender);
                    }
                }
                Err(e) => {
                    e.show_error_dialog(sender);
                }
            }
        });
    }
}
