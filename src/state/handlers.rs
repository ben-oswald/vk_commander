use crate::constants::{
    BANNER_ERROR_DURATION_MS, BANNER_SUCCESS_DURATION_MS, MAX_REQUEST_CHARS, MAX_RESPONSE_CHARS,
    RESP_ERROR_PREFIXES,
};
use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::message::{BannerKind, BannerParams, Event, MainWindow, RespCommand};
use crate::state::{AppState, Message};
use crate::ui::components::{
    BrowserWindow, ConnectionsWindow, DocumentationWindow, ImportWindow, InsightsWindow,
    UIComponents, WorkbenchWindow,
};
use crate::ui::widgets::{Banner, ErrorModal, InfoModal, Popup, PopupType};
use crate::utils::{AppSettings, ValkeyClient, random_string};
use std::sync::{Arc, mpsc::Sender};
use std::thread;

impl AppState {
    pub(crate) fn handle_event(&mut self, e: &Event, ui_components: &mut UIComponents) {
        match e {
            Event::SetMainWindow(main_window) => {
                ui_components.current_window = self.create_window(main_window);
            }
            Event::ShowInfo(i) => {
                self.info = InfoModal::from(i);
            }
            Event::CloseInfo() => {
                if self.info.on_close.is_none() {
                    self.info = InfoModal::default();
                }
            }
            Event::ShowError(s) => {
                self.info.open = false;
                self.error = ErrorModal::from(s);
            }
            Event::AddServer(alias, url) => {
                self.handle_server_operation(|settings| settings.add_server(alias, url));
            }
            Event::DeleteServer(alias) => {
                self.handle_server_operation(|settings| settings.delete_server(alias));
            }
            Event::SetLanguage(language) => {
                self.set_language(language);
            }
            Event::SetConnection(vc) => {
                self.set_vc_client(vc.clone());
            }
            Event::ShowBanner(params) => {
                if let Ok(id) = random_string(12) {
                    self.banners.push(Banner::from_params(id, params));
                }
            }
            Event::DismissBanner(id) => {
                self.banners.retain(|b| b.id() != id);
            }
        }
    }

    pub(crate) fn handle_message(&mut self, message: Message, ui_components: &mut UIComponents) {
        match message {
            Message::Event(e) => self.handle_event(&e, ui_components),
            Message::ToggleSidebar => {
                self.ui_panels.left_side_bar_open = !self.ui_panels.left_side_bar_open;
            }
            Message::OpenPopup(popup_type) => {
                self.create_popup(popup_type);
            }
            Message::ClosePopup(id) => {
                self.popups.retain(|w| w.id != id);
            }
            Message::OpenModal(s) => match random_string(32) {
                Ok(rs) => {
                    self.modals.push(crate::ui::widgets::Modal {
                        id: rs,
                        title: s,
                        open: true,
                    });
                }
                Err(e) => {
                    self.error = ErrorModal::from(e);
                }
            },
            Message::CloseModal(id) => {
                self.modals.retain(|w| w.id != id);
            }
            Message::ExecRespCommand(command) => {
                self.exec_resp_command(command);
            }
            Message::Refresh => {
                if let Some(client) = &self.valkey_client {
                    ui_components.current_window.refresh(client);
                }
            }
        }
    }

    pub(crate) fn create_window(&self, window_type: &MainWindow) -> Box<dyn crate::ui::Component> {
        match window_type {
            MainWindow::Connection => Box::from(ConnectionsWindow::default()),
            MainWindow::Browser => Box::from(BrowserWindow::default()),
            MainWindow::Workbench => Box::from(WorkbenchWindow::default()),
            MainWindow::Insights => Box::from(InsightsWindow::new(
                Arc::new(self.sender.clone()),
                self.i18n.clone(),
            )),
            MainWindow::Documentation => Box::from(DocumentationWindow::default()),
            MainWindow::Import => Box::from(ImportWindow::default()),
        }
    }

    pub(crate) fn handle_server_operation<F>(&mut self, operation: F)
    where
        F: FnOnce(&AppSettings) -> Result<(), Box<Error>>,
    {
        if let Err(e) = operation(&self.settings) {
            self.error = ErrorModal::from(*e);
        } else {
            self.settings.save_to_file().unwrap_or_else(|e| {
                self.error = ErrorModal::from(e);
            });
        }
    }

    pub(crate) fn create_popup(&mut self, popup_type: PopupType) {
        match random_string(32) {
            Ok(id) => {
                let (title, resizable) = match &popup_type {
                    PopupType::AddConnection(_) => (self.i18n.get(LangKey::AddConnection), false),
                    PopupType::EditKey(edit_key) => (
                        format!(
                            "{} - {}",
                            self.i18n.get(LangKey::EditKey),
                            edit_key.key_name()
                        ),
                        true,
                    ),
                    PopupType::AddKey(_) => (self.i18n.get(LangKey::NewKey), true),
                    PopupType::Settings(_) => {
                        self.settings_popup.open = true;
                        return;
                    }
                    PopupType::Undef => return,
                };

                self.popups.push(Popup {
                    id,
                    title,
                    popup_type,
                    open: true,
                    resizable,
                });
            }
            Err(e) => {
                self.error = ErrorModal::from(e);
            }
        }
    }

    fn exec_resp_command(&mut self, command: RespCommand) {
        if let Some(client) = &self.valkey_client {
            let client = client.clone();
            let sender = self.get_sender();
            let handle = thread::spawn(move || {
                Self::process_resp_command(&command, &client, &sender);

                if matches!(command, RespCommand::CommandRefresh(_)) {
                    sender.send(Message::Refresh).unwrap_or_else(|e| {
                        eprintln!("Error sending refresh message: {e}");
                    })
                }
            });
            std::mem::forget(handle);
        }
    }

    fn process_resp_command(
        command: &RespCommand,
        client: &Arc<ValkeyClient>,
        sender: &Arc<Sender<Message>>,
    ) {
        let is_error_like = |s: &str| -> bool {
            RESP_ERROR_PREFIXES.iter().any(|p| s.starts_with(p))
        };

        let cmds = match command {
            RespCommand::Command(cmds) | RespCommand::CommandRefresh(cmds) => cmds,
        };

        let mut maybe_error_msg: Option<String> = None;
        let mut responses: Vec<String> = Vec::new();

        let result = if cmds.len() == 1 {
            cmds.first()
                .map(|c| client.exec(c))
                .unwrap_or(Err(Error::Any("No command to execute".into())))
        } else {
            client.exec_pipelined(cmds)
        };

        match result {
            Ok(res_vec) => {
                if let Some(err) = res_vec.iter().find(|s| is_error_like(s)) {
                    maybe_error_msg = Some(err.clone());
                } else {
                    responses = res_vec;
                }
            }
            Err(e) => {
                maybe_error_msg = Some(e.to_string());
            }
        }

        if let Some(message) = maybe_error_msg {
            let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(
                BannerParams {
                    header: "Server error".into(),
                    message,
                    kind: BannerKind::Error,
                    duration_ms: BANNER_ERROR_DURATION_MS,
                    request: None,
                },
            ))));
        } else {
            let mut response = if responses.is_empty() {
                "OK".to_string()
            } else {
                responses.join("\n")
            };

            if response.len() > MAX_RESPONSE_CHARS {
                response.truncate(MAX_RESPONSE_CHARS);
                response.push_str("\n…");
            }

            let header = match command {
                RespCommand::CommandRefresh(_) => "Success",
                RespCommand::Command(_) => "Successfully updated",
            };

            let mut request = cmds.join(" ");
            if request.len() > MAX_REQUEST_CHARS {
                request.truncate(MAX_REQUEST_CHARS);
                request.push_str("[...]");
            }

            let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(
                BannerParams {
                    header: header.into(),
                    message: response,
                    kind: BannerKind::Success,
                    duration_ms: BANNER_SUCCESS_DURATION_MS,
                    request: Some(request),
                },
            ))));
        }
    }
}