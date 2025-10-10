use crate::errors::Error;
use crate::i18n::I18N;
use crate::state;
use crate::state::Message;
use crate::utils::ValkeyClient;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub mod components;
pub mod widgets;

pub trait View {
    fn ui(ui: &mut egui::Ui, state: &mut state::AppState);
}

pub trait Component {
    fn show(&mut self, ctx: &egui::Context, state: &mut state::AppState) -> Result<(), Error>;
    fn refresh(&mut self, client: &Arc<ValkeyClient>);
}

pub trait Dialog {
    fn show(&self, ctx: &egui::Context, i18n: Arc<I18N>) -> Result<(), Error>;
}

pub trait Widget {
    fn show(
        &mut self,
        ctx: &egui::Context,
        sender: Arc<Sender<Message>>,
        i18n: Arc<I18N>,
        collapsable: bool,
        resizable: bool,
    ) -> Result<(), Error>;
}
