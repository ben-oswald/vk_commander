#[derive(Clone, Copy, Default, PartialEq)]
pub enum ResultViewMode {
    #[default]
    Text,
    Table,
}

#[derive(Default)]
pub struct WorkbenchState {
    pub resp_command: String,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    pub temp_command: String,
    pub autocomplete_selected_index: usize,
    pub show_autocomplete: bool,
    pub result_data: Vec<String>,
    pub view_mode: ResultViewMode,
    pub set_cursor_pos: Option<usize>,
    pub is_multiline: bool,
}
