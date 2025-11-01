mod add_connection_popup;
mod add_key;
mod edit_key;
mod settings_popup;

use crate::i18n::I18N;
use crate::state::Message;
pub use add_connection_popup::AddConnectionPopup;
pub use add_key::AddKey;
pub use edit_key::EditKey;
use egui::text_edit::TextEditOutput;
use egui_code_editor::CodeEditor;
pub use settings_popup::SettingsPopup;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub trait PopupUi {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    );
}

fn code_editor(ui: &mut egui::Ui, text: &mut String) -> TextEditOutput {
    CodeEditor::default()
        .id_source("json_editor")
        .with_numlines(false)
        .with_rows(16)
        .vscroll(true)
        .stick_to_bottom(true)
        .show(ui, text)
}
