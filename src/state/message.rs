use crate::errors::Error;
use crate::i18n::Language;
use crate::ui::widgets::PopupType;
use crate::utils::ValkeyClient;
use std::sync::Arc;

pub enum Message {
    Event(Arc<Event>),
    ToggleSidebar,
    OpenPopup(PopupType),
    ClosePopup(String),
    OpenModal(String),
    CloseModal(String),
    ExecRespCommand(RespCommand),
    Refresh,
}

pub enum RespCommand {
    Command(Vec<String>),
    CommandRefresh(Vec<String>),
}

pub enum Event {
    SetMainWindow(MainWindow),
    ShowInfo(Info),
    CloseInfo(),
    ShowError(Error),
    AddServer(String, String),
    DeleteServer(String),
    SetLanguage(Language),
    SetConnection(Arc<ValkeyClient>),
    ShowBanner(BannerParams),
    DismissBanner(String), // id
}

#[derive(Clone, Copy)]
pub enum MainWindow {
    Connection,
    Browser,
    Workbench,
    Insights,
    Documentation,
}

pub struct Info {
    pub title: String,
    pub message: String,
    pub callback: Option<fn()>,
}

#[derive(Clone, Copy)]
pub enum BannerKind {
    Info,
    Error,
    Success,
}

pub struct BannerParams {
    pub header: String,
    pub message: String,
    pub kind: BannerKind,
    /// Duration in milliseconds the banner stays visible before auto-dismiss.
    pub duration_ms: u64,
    /// Request text to display alongside the response [optional]
    pub request: Option<String>,
}
