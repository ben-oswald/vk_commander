use crate::errors::Error;
use crate::i18n::{I18N, Language};
use crate::state::message::MainWindow;
use crate::state::workbench_state::WorkbenchState;
use crate::ui::components::UIPanels;
use crate::ui::widgets::{Banner, ErrorModal, InfoModal, Modal, Popup, PopupType, SettingsPopup};
use crate::utils::{AppSettings, CommandRegistry, get_commands_dir};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, RwLock};

pub struct AppState {
    pub ui_panels: UIPanels,
    pub popups: Vec<Popup>,
    pub settings_popup: Popup,
    pub modals: Vec<Modal>,
    pub info: InfoModal,
    pub error: ErrorModal,
    pub show_about: bool,
    pub valkey_client: Option<Arc<crate::utils::ValkeyClient>>,
    pub resizable: bool,
    pub workbench_state: WorkbenchState,
    pub command_registry: Arc<CommandRegistry>,
    pub banners: Vec<Banner>,
    pub(crate) i18n: Arc<I18N>,
    pub(crate) settings: Arc<AppSettings>,
    pub(crate) sender: Sender<super::Message>,
    receiver: Receiver<super::Message>,
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
                title: i18n.get(crate::i18n::LangKey::Settings),
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
            banners: Vec::new(),
        }
    }

    pub fn get_state(&mut self, ui_components: &mut crate::ui::components::UIComponents) {
        while let Ok(message) = self.receiver.try_recv() {
            self.handle_message(message, ui_components);
        }
    }

    pub fn set_state(&mut self, msg: super::Message) {
        if let Err(e) = self.sender.send(msg) {
            self.error = ErrorModal::from(Error::from(e));
        }
    }

    pub fn get_sender(&self) -> Arc<Sender<super::Message>> {
        Arc::new(self.sender.clone())
    }

    pub fn i18n(&self) -> Arc<I18N> {
        self.i18n.clone()
    }

    pub fn set_language(&mut self, language: &Language) {
        self.i18n = Arc::new(I18N::new(*language));
    }

    pub fn set_vc_client(&mut self, client: Arc<crate::utils::ValkeyClient>) {
        self.ui_panels.current_window = Arc::new(RwLock::new(Some(MainWindow::Browser)));
        self.valkey_client = Some(client);
    }

    pub fn get_settings(&self) -> Arc<AppSettings> {
        Arc::clone(&self.settings)
    }
}