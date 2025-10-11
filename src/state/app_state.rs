use crate::errors::Error;
use crate::i18n::{I18N, LangKey, Language};
use crate::state::Message;
use crate::state::message::{Event, MainWindow, RespCommand};
use crate::state::workbench_state::WorkbenchState;
use crate::ui::components;
use crate::ui::components::{ConnectionsWindow, UIComponents, UIPanels};
use crate::ui::widgets::{ErrorModal, InfoModal, Modal, Popup, PopupType, SettingsPopup};
use crate::utils::{AppSettings, CommandRegistry, ValkeyClient, get_commands_dir, random_string};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, RwLock};
use std::thread;

pub struct AppState {
    pub ui_panels: UIPanels,
    pub popups: Vec<Popup>,
    pub settings_popup: Popup,
    pub modals: Vec<Modal>,
    pub info: InfoModal,
    pub error: ErrorModal,
    pub show_about: bool,
    pub valkey_client: Option<Arc<ValkeyClient>>,
    pub resizable: bool,
    pub workbench_state: WorkbenchState,
    pub command_registry: Arc<CommandRegistry>,
    i18n: Arc<I18N>,
    settings: Arc<AppSettings>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(false)
    }
}

impl AppState {
    pub fn new(resizable: bool) -> Self {
        let (tx, rx) = channel();
        let settings = Arc::new(AppSettings::new_from_file());
        let language = settings.get_language();
        let i18n = Arc::new(I18N::new(language));

        let command_registry = CommandRegistry::load_from_directory(get_commands_dir())
            .unwrap_or_else(|_| CommandRegistry::default());

        Self {
            ui_panels: UIPanels {
                left_side_bar_open: true,
                current_window: Arc::new(Default::default()),
            },
            popups: Vec::new(),
            settings_popup: Popup {
                id: "settings".to_string(),
                title: i18n.get(LangKey::Settings),
                popup_type: PopupType::Settings(Box::new(SettingsPopup::new(language))),
                resizable: true,
                open: false,
            },
            modals: Vec::new(),
            info: Default::default(),
            error: ErrorModal::default(),
            show_about: false,
            valkey_client: None,
            i18n,
            settings,
            sender: tx,
            receiver: rx,
            resizable,
            workbench_state: Default::default(),
            command_registry: Arc::new(command_registry),
        }
    }

    pub fn get_state(&mut self, ui_components: &mut UIComponents) {
        if let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::Event(e) => match &*e {
                    Event::SetMainWindow(main_window) => {
                        ui_components.current_window = self.create_window(main_window);
                    }
                    Event::ShowInfo(i) => {
                        self.info = InfoModal::from(i);
                    }
                    Event::CloseInfo() => {
                        self.info = InfoModal::default();
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
                },
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
                        self.modals.push(Modal {
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
                    if let Some(client) = &self.valkey_client {
                        let client = client.clone();
                        let sender = self.get_sender();
                        let handle = thread::spawn(move || {
                            //TODO! Impl
                            match &command {
                                RespCommand::Command(cmds) | RespCommand::CommandRefresh(cmds) => {
                                    if cmds.len() == 1 {
                                        //Safely unwrap
                                        let res =
                                            client.exec(cmds.first().unwrap()).unwrap_or_default();
                                    } else {
                                        let res = client.exec_pipelined(cmds);
                                    }
                                    if matches!(command, RespCommand::CommandRefresh(_)) {
                                        sender.send(Message::Refresh).unwrap_or_else(|e| {
                                            eprintln!("Error sending refresh message: {e}");
                                        })
                                    }
                                }
                            }
                        });
                        std::mem::forget(handle);
                    }
                }
                Message::Refresh => {
                    if let Some(client) = &self.valkey_client {
                        ui_components.current_window.refresh(client);
                    }
                }
            }
        };
    }

    pub fn set_state(&mut self, msg: Message) {
        if let Err(e) = self.sender.send(msg) {
            self.error = ErrorModal::from(Error::from(e));
        }
    }

    pub fn get_sender(&self) -> Arc<Sender<Message>> {
        Arc::new(self.sender.clone())
    }

    pub fn i18n(&self) -> Arc<I18N> {
        self.i18n.clone()
    }

    pub fn set_language(&mut self, language: &Language) {
        self.i18n = Arc::new(I18N::new(*language));
    }

    pub fn set_vc_client(&mut self, client: Arc<ValkeyClient>) {
        self.ui_panels.current_window = Arc::new(RwLock::new(Some(MainWindow::Browser)));
        self.valkey_client = Some(client);
    }

    pub fn get_settings(&self) -> Arc<AppSettings> {
        Arc::clone(&self.settings)
    }

    fn create_window(&self, window_type: &MainWindow) -> Box<dyn crate::ui::Component> {
        match window_type {
            MainWindow::Connection => Box::from(ConnectionsWindow::default()),
            MainWindow::Browser => Box::from(components::BrowserWindow::default()),
            MainWindow::Workbench => Box::from(components::WorkbenchWindow::default()),
            MainWindow::Insights => Box::from(components::InsightsWindow::new(
                Arc::new(self.sender.clone()),
                self.i18n.clone(),
            )),
            MainWindow::Documentation => Box::from(components::DocumentationWindow::default()),
        }
    }

    fn handle_server_operation<F>(&mut self, operation: F)
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

    fn create_popup(&mut self, popup_type: PopupType) {
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
}
