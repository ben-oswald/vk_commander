mod add_connection_popup;
mod add_key;
mod edit_key;
mod settings_popup;

use crate::state::Message;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::i18n::I18N;
pub use add_connection_popup::AddConnectionPopup;
pub use add_key::AddKey;
pub use edit_key::EditKey;
pub use settings_popup::SettingsPopup;

pub trait PopupUi {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    );
}
