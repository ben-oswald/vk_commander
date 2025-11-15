use crate::errors::Error;
use crate::i18n::{I18N, LangKey};
use crate::state::{BannerKind, BannerParams, Event, Message, RespCommand};
use crate::ui::widgets::popups::{PopupUi, code_editor};
use crate::utils::{KeyType, format_size, text_float_filter};
use egui::{Key, Ui};
use egui_extras::{Column, TableBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Sender;

const COL_HEIGHT: f32 = 28.0;

pub struct EditKey {
    key: String,
    key_type: KeyType,
    data: Vec<(String, String)>,
    original_data: Vec<(String, String)>,
    focused_cell: Option<(usize, usize)>,
    original_values: HashMap<(usize, usize), String>,
    rows_to_delete: Vec<usize>,
    new_field_name: String,
    new_field_value: String,
    show_add_form: bool,
    i18n: Arc<I18N>,
    has_unsaved_changes: bool,
    show_unsaved_dialog: bool,
}

impl EditKey {
    pub fn new(key: String, key_type: KeyType, data: Vec<String>, i18n: Arc<I18N>) -> Self {
        let data_vec: Vec<(String, String)> = match key_type {
            KeyType::Hash | KeyType::SortedSet | KeyType::Bloom => data
                .iter()
                .enumerate()
                .step_by(2)
                .map(|(index, value)| {
                    (
                        value.to_string(),
                        data.get(index + 1).unwrap_or(&"".to_string()).to_string(),
                    )
                })
                .collect(),
            KeyType::Set => data
                .iter()
                .map(|value| (value.to_string(), "".to_string()))
                .collect(),
            KeyType::List => data
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value.to_string()))
                .collect(),
            KeyType::String => {
                if !data.is_empty() {
                    vec![("".to_string(), data[0].to_string())]
                } else {
                    vec![("".to_string(), "".to_string())]
                }
            }
            KeyType::Json => {
                if !data.is_empty() {
                    let prettified = serde_json::from_str::<serde_json::Value>(&data[0])
                        .map(|v| {
                            serde_json::to_string_pretty(&v).unwrap_or_else(|e| {
                                Error::from(e).log_error();
                                data[0].clone()
                            })
                        })
                        .unwrap_or_else(|e| {
                            Error::from(e).log_error();
                            data[0].clone()
                        });
                    vec![("".to_string(), prettified)]
                } else {
                    vec![("".to_string(), "".to_string())]
                }
            }
        };

        Self {
            key,
            key_type,
            original_data: data_vec.clone(),
            data: data_vec,
            focused_cell: None,
            original_values: HashMap::new(),
            rows_to_delete: vec![],
            new_field_name: String::new(),
            new_field_value: String::new(),
            show_add_form: false,
            i18n,
            has_unsaved_changes: false,
            show_unsaved_dialog: false,
        }
    }

    pub fn key_name(&self) -> &str {
        &self.key
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    pub fn open_unsaved_changes_dialog(&mut self) {
        if self.has_unsaved_changes {
            self.show_unsaved_dialog = true;
        }
    }

    fn mark_changed(&mut self) {
        self.has_unsaved_changes = true;
    }

    fn delete_button(ui: &mut Ui) -> bool {
        if ui.small_button("🗑").clicked() {
            true
        } else {
            false
        }
    }

    fn highlighted_column(
        (key, key_type): (&str, &KeyType),
        ui: &mut Ui,
        (row, index): ((&String, &mut String), usize),
        (read_only, is_focused): (bool, bool),
        cell_id: (usize, usize),
        original_values: &mut HashMap<(usize, usize), String>,
        (_sender, _i18n): (&Arc<Sender<Message>>, &I18N),
    ) -> egui::Response {
        let max_rect = ui.max_rect();
        let highlight_rect =
            egui::Rect::from_min_size(max_rect.min, egui::Vec2::new(max_rect.width(), COL_HEIGHT));

        let is_hovered = ui.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|pos| highlight_rect.contains(pos))
        });

        if is_focused {
            ui.painter().rect_filled(
                highlight_rect,
                0.0,
                ui.style().visuals.selection.bg_fill.gamma_multiply(0.5),
            );
        } else if is_hovered {
            ui.painter().rect_filled(
                highlight_rect,
                0.0,
                ui.style()
                    .visuals
                    .widgets
                    .hovered
                    .bg_fill
                    .gamma_multiply(0.3),
            );
        }

        if read_only {
            if matches!(key_type, KeyType::List) {
                ui.label(format!("[{}]", row.0))
            } else {
                ui.label(row.0.as_str())
            }
        } else {
            ui.style_mut().visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;

            let text_edit = egui::TextEdit::singleline(row.1)
                .desired_width(f32::INFINITY)
                .frame(false)
                .margin(egui::Margin::ZERO);

            let response = ui.add(text_edit);

            if response.gained_focus() {
                original_values.insert(cell_id, row.1.clone());
            }
            if response.changed() && matches!(key_type, KeyType::SortedSet) {
                *row.1 = text_float_filter(row.1)
            }
            response
        }
    }
    fn input_field(&mut self, ui: &mut Ui, _sender: &Arc<Sender<Message>>) {
        if let Some(&mut (_, ref mut value)) = self.data.first_mut() {
            let mut changed = false;

            egui::ScrollArea::vertical()
                .id_salt("edit_key_input_field_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    let response = if matches!(self.key_type, KeyType::Json | KeyType::String) {
                        code_editor(ui, value).response
                    } else {
                        let text_edit = egui::TextEdit::multiline(value).desired_rows(10);
                        ui.add(text_edit)
                    };

                    if response.changed() {
                        changed = true;
                    }
                });

            if changed {
                self.mark_changed();
            }
        } else {
            ui.label(self.i18n.get(LangKey::NoData));
        };
    }

    fn delete_marked_rows(&mut self) {
        let mut any_deleted = false;
        for &row_index in self.rows_to_delete.iter().rev() {
            self.data.remove(row_index);
            any_deleted = true;

            if let Some((focused_row, focused_col)) = self.focused_cell {
                if focused_row == row_index {
                    self.focused_cell = None;
                } else if focused_row > row_index {
                    self.focused_cell = Some((focused_row - 1, focused_col));
                }
            }

            let keys_to_remove: Vec<_> = self
                .original_values
                .keys()
                .filter(|(row, _)| *row == row_index)
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.original_values.remove(&key);
            }

            let mut updated_values = std::collections::HashMap::new();
            for ((row, col), value) in self.original_values.drain() {
                if row > row_index {
                    updated_values.insert((row - 1, col), value);
                } else if row < row_index {
                    updated_values.insert((row, col), value);
                }
            }
            self.original_values = updated_values;
        }

        if any_deleted {
            self.mark_changed();
        }
    }

    fn add_new_field_form(&mut self, ui: &mut Ui, sender: &Arc<Sender<Message>>) {
        ui.horizontal(|ui| {
            ui.label(self.i18n.get(LangKey::AddNew));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.i18n.get(LangKey::Cancel)).clicked() {
                    self.show_add_form = false;
                    self.new_field_name.clear();
                    self.new_field_value.clear();
                }

                let double_field_name = self.data.iter().any(|d| d.0 == self.new_field_name);

                let add_enabled = match self.key_type {
                    KeyType::Set => !self.new_field_name.is_empty() && !double_field_name,
                    KeyType::List => !self.new_field_value.is_empty(),
                    KeyType::Hash => {
                        !self.new_field_name.is_empty()
                            && !self.new_field_value.is_empty()
                            && !double_field_name
                    }
                    KeyType::SortedSet => {
                        !self.new_field_name.is_empty()
                            && !self.new_field_value.is_empty()
                            && self.new_field_value.parse::<f64>().is_ok()
                            && !double_field_name
                    }
                    KeyType::String => false,
                    KeyType::Json => false,
                    KeyType::Bloom => false,
                };

                if ui
                    .add_enabled(add_enabled, egui::Button::new(self.i18n.get(LangKey::Add)))
                    .clicked()
                {
                    self.add_new_field();
                }
            });
        });

        ui.add_space(5.0);

        self.key_edit_field(ui);
    }

    fn add_new_field(&mut self) {
        if self.new_field_name.is_empty()
            && matches!(
                self.key_type,
                KeyType::Hash | KeyType::Set | KeyType::SortedSet
            )
        {
            return;
        }
        if self.new_field_value.is_empty()
            && matches!(
                self.key_type,
                KeyType::Hash | KeyType::List | KeyType::SortedSet
            )
        {
            return;
        }

        match self.key_type {
            KeyType::Hash => {
                self.data
                    .push((self.new_field_name.clone(), self.new_field_value.clone()));
            }
            KeyType::Set => {
                self.data
                    .push((self.new_field_name.clone(), "".to_string()));
            }
            KeyType::SortedSet => {
                if self.new_field_value.parse::<f64>().is_err() {
                    eprintln!("Invalid score for sorted set: {}", self.new_field_value);
                    return;
                }
                self.data
                    .push((self.new_field_name.clone(), self.new_field_value.clone()));
            }
            KeyType::List => {
                let new_index = self.data.len();
                self.data
                    .push((new_index.to_string(), self.new_field_value.clone()));
            }
            KeyType::String | KeyType::Json | KeyType::Bloom => {
                return;
            }
        }

        self.new_field_name.clear();
        self.new_field_value.clear();
        self.show_add_form = false;
        self.mark_changed();
    }

    fn data_table(&mut self, ui: &mut Ui, sender: &Arc<Sender<Message>>) {
        self.delete_marked_rows();
        self.rows_to_delete.clear();

        let available_height = ui.available_height();
        let form_height = if self.show_add_form { 100.0 } else { 50.0 };
        let table_height = (available_height - form_height).max(200.0);

        let mut any_row_changed = false;

        egui::ScrollArea::vertical()
            .id_salt("edit_key_data_table_scroll")
            .max_height(table_height)
            .show(ui, |ui| {
                let table_builder = TableBuilder::new(ui);

                let table_builder = if matches!(self.key_type, KeyType::Hash | KeyType::SortedSet) {
                    table_builder
                        .column(Column::initial(150.0).range(100.0..=500.0).resizable(true))
                        .column(Column::remainder().at_least(100.0))
                        .column(Column::exact(30.0))
                } else if matches!(self.key_type, KeyType::List) {
                    table_builder
                        .column(Column::initial(60.0).range(40.0..=100.0).resizable(true))
                        .column(Column::remainder().at_least(200.0))
                        .column(Column::exact(30.0))
                } else {
                    table_builder
                        .column(Column::remainder().at_least(200.0))
                        .column(Column::exact(30.0))
                };

                table_builder
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .striped(true)
                    .header(20.0, |mut header| {
                        if matches!(
                            self.key_type,
                            KeyType::Hash | KeyType::SortedSet | KeyType::Set | KeyType::List
                        ) {
                            header.col(|ui| {
                                ui.strong(match self.key_type {
                                    KeyType::SortedSet | KeyType::Set => "Member",
                                    KeyType::List => "Index",
                                    _ => "Key",
                                });
                            });
                            if !matches!(self.key_type, KeyType::Set) {
                                header.col(|ui| {
                                    ui.strong(match self.key_type {
                                        KeyType::SortedSet => "Score",
                                        _ => "Value",
                                    });
                                });
                            }
                        }
                    })
                    .body(|mut body| {
                        for (row_index, row) in self.data.iter_mut().enumerate() {
                            body.row(COL_HEIGHT, |mut row_ui| {
                                if matches!(
                                    self.key_type,
                                    KeyType::Hash
                                        | KeyType::SortedSet
                                        | KeyType::Set
                                        | KeyType::List
                                ) {
                                    row_ui.col(|ui| {
                                        let is_focused = self.focused_cell == Some((row_index, 0));

                                        Self::highlighted_column(
                                            (&self.key, &self.key_type),
                                            ui,
                                            ((&row.0, &mut row.1), row_index),
                                            (true, is_focused),
                                            (row_index, 0),
                                            &mut self.original_values,
                                            (sender, &self.i18n),
                                        );
                                    });
                                }
                                if !matches!(self.key_type, KeyType::Set) {
                                    row_ui.col(|ui| {
                                        let is_focused = self.focused_cell == Some((row_index, 1));

                                        let response = Self::highlighted_column(
                                            (&self.key, &self.key_type),
                                            ui,
                                            ((&row.0, &mut row.1), row_index),
                                            (false, is_focused),
                                            (row_index, 1),
                                            &mut self.original_values,
                                            (sender, &self.i18n),
                                        );

                                        if response.changed() {
                                            any_row_changed = true;
                                        }

                                        if response.has_focus() {
                                            self.focused_cell = Some((row_index, 1));
                                        } else if self.focused_cell == Some((row_index, 1)) {
                                            self.focused_cell = None;
                                        }
                                    });
                                }
                                row_ui.col(|ui| {
                                    if Self::delete_button(ui) {
                                        self.rows_to_delete.push(row_index);
                                    }
                                });
                            });
                        }
                    });
            });

        if any_row_changed {
            self.mark_changed();
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        if !self.show_add_form {
            let save_label = self.i18n.get(LangKey::Save).to_string();
            let add_label = format!("➕ {}", self.i18n.get(LangKey::AddNew));

            let mut save_clicked = false;
            let mut toggle_add_form = false;

            egui::Sides::new().show(
                ui,
                |ui| {
                    if ui.button(add_label).clicked() {
                        toggle_add_form = true;
                    }
                },
                |ui| {
                    if ui
                        .add_enabled(
                            self.has_unsaved_changes,
                            egui::Button::new(save_label),
                        )
                        .clicked()
                    {
                        save_clicked = true;
                    }
                },
            );

            if save_clicked {
                self.save_changes(sender);
            }

            if toggle_add_form {
                self.show_add_form = !self.show_add_form;
            }
        }


        if self.show_add_form {
            self.add_new_field_form(ui, sender);
        }
    }

    fn save_changes(&mut self, sender: &Arc<Sender<Message>>) {
        if !self.has_unsaved_changes {
            return;
        }

        let key_escaped = self.key.replace('"', "\\\"");
        let mut commands: Vec<String> = Vec::new();

        match self.key_type {
            KeyType::Hash => {
                use std::collections::HashMap;

                let mut original: HashMap<&String, &String> = HashMap::new();
                for (field, value) in &self.original_data {
                    original.insert(field, value);
                }

                let mut current: HashMap<&String, &String> = HashMap::new();
                for (field, value) in &self.data {
                    current.insert(field, value);
                }

                let mut removed_fields: Vec<String> = original
                    .keys()
                    .filter(|f| !current.contains_key(*f))
                    .map(|f| f.replace('"', "\\\""))
                    .collect();
                if !removed_fields.is_empty() {
                    let mut cmd = format!("HDEL \"{}\"", key_escaped);
                    for field in removed_fields.drain(..) {
                        cmd.push_str(&format!(" \"{}\"", field));
                    }
                    commands.push(cmd);
                }

                let mut set_pairs: Vec<(String, String)> = Vec::new();
                for (field, value) in &self.data {
                    match original.get(field) {
                        Some(orig_v) if *orig_v == value => {}
                        _ => {
                            set_pairs
                                .push((field.replace('"', "\\\""), value.replace('"', "\\\"")));
                        }
                    }
                }

                if !set_pairs.is_empty() {
                    let mut cmd = format!("HSET \"{}\"", key_escaped);
                    for (field, value) in set_pairs.drain(..) {
                        cmd.push_str(&format!(" \"{}\" \"{}\"", field, value));
                    }
                    commands.push(cmd);
                }
            }
            KeyType::List => {
                commands.push(format!("DEL \"{}\"", key_escaped));
                if !self.data.is_empty() {
                    let mut cmd = format!("RPUSH \"{}\"", key_escaped);
                    for (_, value) in &self.data {
                        cmd.push_str(&format!(" \"{}\"", value.replace('"', "\\\""),));
                    }
                    commands.push(cmd);
                }
            }
            KeyType::Set => {
                let original_set: HashSet<String> =
                    self.original_data.iter().map(|(m, _)| m.clone()).collect();
                let current_set: HashSet<String> =
                    self.data.iter().map(|(m, _)| m.clone()).collect();

                let removed: Vec<String> = original_set.difference(&current_set).cloned().collect();
                if !removed.is_empty() {
                    let mut cmd = format!("SREM \"{}\"", key_escaped);
                    for member in removed {
                        cmd.push_str(&format!(" \"{}\"", member.replace('"', "\\\""),));
                    }
                    commands.push(cmd);
                }

                let added: Vec<String> = current_set.difference(&original_set).cloned().collect();
                if !added.is_empty() {
                    let mut cmd = format!("SADD \"{}\"", key_escaped);
                    for member in added {
                        cmd.push_str(&format!(" \"{}\"", member.replace('"', "\\\""),));
                    }
                    commands.push(cmd);
                }
            }
            KeyType::SortedSet => {
                commands.push(format!("DEL \"{}\"", key_escaped));
                if !self.data.is_empty() {
                    let mut cmd = format!("ZADD \"{}\"", key_escaped);
                    for (member, score) in &self.data {
                        if score.parse::<f64>().is_ok() {
                            cmd.push_str(&format!(
                                " {} \"{}\"",
                                score,
                                member.replace('"', "\\\""),
                            ));
                        } else {
                            eprintln!("Invalid score for sorted set: {}", score);
                        }
                    }
                    commands.push(cmd);
                }
            }
            KeyType::String => {
                if let Some((_, value)) = self.data.first() {
                    let original_value = self
                        .original_data
                        .first()
                        .map(|(_, v)| v)
                        .cloned()
                        .unwrap_or_default();

                    if &original_value != value {
                        commands.push(format!(
                            "SET \"{}\" \"{}\"",
                            key_escaped,
                            value.replace('"', "\\\""),
                        ));
                    }
                }
            }
            KeyType::Json => {
                if let Some((_, value)) = self.data.first() {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(
                            BannerParams {
                                header: "Invalid JSON".into(),
                                message: "JSON value cannot be empty".into(),
                                kind: BannerKind::Error,
                                duration_ms: 10_000,
                                request: None,
                            },
                        ))));
                        return;
                    }

                    if let Err(e) = serde_json::from_str::<serde_json::Value>(value) {
                        let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(
                            BannerParams {
                                header: "Invalid JSON".into(),
                                message: e.to_string(),
                                kind: BannerKind::Error,
                                duration_ms: 10_000,
                                request: None,
                            },
                        ))));
                        return;
                    }

                    let escaped_double = value.replace('"', "\\\"");
                    commands.push(format!(
                        "JSON.SET \"{}\" $ \"{}\"",
                        key_escaped, escaped_double,
                    ));
                }
            }
            KeyType::Bloom => {
                // Bloom filters are currently read-only in this editor.
            }
        }

        if !commands.is_empty() {
            sender
                .send(Message::ExecRespCommand(RespCommand::Command(commands)))
                .unwrap_or_else(|e| {
                    eprintln!("Error sending message: {e}");
                });
        }

        self.original_data = self.data.clone();
        self.has_unsaved_changes = false;
        self.original_values.clear();
    }

    fn bloom_filter(&self, ui: &mut Ui, sender: &Arc<Sender<Message>>) {
        let bf_info: HashMap<String, String> = self.data.clone().into_iter().collect();

        ui.vertical(|ui| {
            ui.heading(self.i18n.get(LangKey::BloomFilterInformation));
            ui.add_space(10.0);

            let ordered_fields = [
                (
                    "Number of items inserted",
                    self.i18n.get(LangKey::Items),
                    "📊",
                ),
                ("Capacity", self.i18n.get(LangKey::Capacity), "🗄️"),
                (
                    "Max scaled capacity",
                    self.i18n.get(LangKey::MaxCapacity),
                    "📈",
                ),
                ("Number of filters", self.i18n.get(LangKey::Filters), "🔍"),
                ("Size", self.i18n.get(LangKey::Size), "💾"),
                ("Error rate", self.i18n.get(LangKey::ErrorRate), "⚠️"),
                ("Expansion rate", self.i18n.get(LangKey::Expansion), "📊"),
                ("Tightening ratio", self.i18n.get(LangKey::Tightening), "🔧"),
            ];

            let available_width = ui.available_width();
            let card_width = 110.0;
            let spacing = 8.0;

            let valid_fields: Vec<_> = ordered_fields
                .iter()
                .filter(|(raw_key, _, _)| bf_info.contains_key(*raw_key))
                .collect();

            let mut field_index = 0;

            while field_index < valid_fields.len() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;

                    let mut temp_x = 0.0;
                    let mut cards_in_this_row = 0;

                    while field_index + cards_in_this_row < valid_fields.len()
                        && temp_x + card_width <= available_width
                    {
                        temp_x += card_width + spacing;
                        cards_in_this_row += 1;
                    }

                    for i in 0..cards_in_this_row {
                        let (key, display_name, icon) = valid_fields[field_index + i];

                        if let Some(value) = bf_info.get(*key) {
                            egui::Frame::NONE
                                .fill(ui.style().visuals.faint_bg_color)
                                .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
                                .corner_radius(4.0)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.set_width(card_width - 16.0);
                                    ui.set_height(60.0);

                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new(*icon).size(16.0));
                                        ui.add_space(2.0);

                                        let short_name = if *display_name
                                            == self.i18n.get(LangKey::MaxCapacity)
                                        {
                                            self.i18n.get(LangKey::MaxCap)
                                        } else if *display_name == self.i18n.get(LangKey::ErrorRate)
                                        {
                                            self.i18n.get(LangKey::Error)
                                        } else if *display_name
                                            == self.i18n.get(LangKey::Tightening)
                                        {
                                            self.i18n.get(LangKey::Tight)
                                        } else {
                                            display_name.clone()
                                        };

                                        ui.label(
                                            egui::RichText::new(short_name)
                                                .strong()
                                                .size(9.5)
                                                .color(ui.style().visuals.weak_text_color()),
                                        );

                                        ui.add_space(1.0);

                                        let formatted_value = match *key {
                                            "Size" | "Max scaled capacity" => {
                                                if let Ok(num) = value.parse::<u64>() {
                                                    format_size(num)
                                                } else {
                                                    value.clone()
                                                }
                                            }
                                            "Error rate" => format!(
                                                "{:.2}%",
                                                (value.parse::<f64>().unwrap_or(0.0) * 100.0)
                                            ),
                                            "Number of items inserted" | "Capacity" => {
                                                value.clone()
                                            }
                                            "Expansion rate" | "Tightening ratio" => {
                                                if let Ok(num) = value.parse::<f64>() {
                                                    format!("{num:.1}")
                                                } else {
                                                    value.clone()
                                                }
                                            }
                                            _ => value.clone(),
                                        };

                                        ui.label(
                                            egui::RichText::new(formatted_value)
                                                .strong()
                                                .size(12.0),
                                        );
                                    });
                                });
                        }
                    }

                    field_index += cards_in_this_row;
                });

                ui.add_space(8.0);
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(self.i18n.get(LangKey::Summary));
                ui.separator();

                if let (Some(items), Some(capacity)) = (
                    bf_info
                        .get("Number of items inserted")
                        .and_then(|s| s.parse::<u64>().ok()),
                    bf_info.get("Capacity").and_then(|s| s.parse::<u64>().ok()),
                ) {
                    let fill_percentage = if capacity > 0 {
                        (items as f64 / capacity as f64 * 100.0).min(100.0)
                    } else {
                        0.0
                    };

                    ui.label(format!(
                        "{}: {fill_percentage:.1}%",
                        self.i18n.get(LangKey::Fill)
                    ));
                    ui.separator();
                }

                if let Some(error_rate) = bf_info.get("Error rate") {
                    ui.label(format!(
                        "{}: {:.3}%",
                        self.i18n.get(LangKey::Error),
                        error_rate.parse::<f64>().unwrap_or(0.0) * 100.0
                    ));
                    ui.separator();
                }

                if let Some(size) = bf_info.get("Size").and_then(|s| s.parse::<u64>().ok()) {
                    ui.label(format!(
                        "{}: {}",
                        self.i18n.get(LangKey::Size),
                        format_size(size)
                    ));
                }
            });
        });
    }

    fn key_edit_field(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut col0_has_focus = false;
            let mut col1_has_focus = false;

            let double_layout = matches!(&self.key_type, KeyType::Hash | KeyType::SortedSet);

            ui.vertical(|ui| {
                col0_has_focus = ui
                    .add(
                        egui::TextEdit::singleline(if matches!(&self.key_type, KeyType::List) {
                            &mut self.new_field_value
                        } else {
                            &mut self.new_field_name
                        })
                        .hint_text(
                            if matches!(&self.key_type, KeyType::List) {
                                self.i18n.get(LangKey::Element)
                            } else if matches!(&self.key_type, KeyType::Set | KeyType::SortedSet) {
                                self.i18n.get(LangKey::Member)
                            } else {
                                self.i18n.get(LangKey::Key)
                            },
                        ),
                    )
                    .has_focus();
            });

            if double_layout {
                ui.vertical(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.new_field_value)
                            .desired_width(f32::INFINITY)
                            .hint_text(if matches!(&self.key_type, KeyType::SortedSet) {
                                self.i18n.get(LangKey::Score)
                            } else {
                                self.i18n.get(LangKey::Value)
                            }),
                    );

                    col1_has_focus = response.has_focus();
                    if response.changed() && matches!(&self.key_type, KeyType::SortedSet) {
                        self.new_field_value = text_float_filter(&self.new_field_value)
                    }
                });
            }
        });
    }
}

impl PopupUi for EditKey {
    fn ui(&mut self, ui: &mut Ui, sender: &Arc<Sender<Message>>, _: &Arc<I18N>, open: &mut bool) {
        ui.vertical(|ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.heading(self.key_type.to_string());
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);

                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(Key::W)) {
                    if self.has_unsaved_changes {
                        self.show_unsaved_dialog = true;
                    } else {
                        *open = false;
                    }
                }

                if matches!(self.key_type, KeyType::String | KeyType::Json) {
                    self.input_field(ui, sender);

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        let save_label = self.i18n.get(LangKey::Save);
                        if ui
                            .add_enabled(self.has_unsaved_changes, egui::Button::new(save_label))
                            .clicked()
                        {
                            self.save_changes(sender);
                        }
                    });
                } else if matches!(self.key_type, KeyType::Bloom) {
                    self.bloom_filter(ui, sender);
                } else {
                    self.data_table(ui, sender);
                }

                if self.show_unsaved_dialog {
                    egui::Modal::new(egui::Id::new("edit_key_unsaved_changes")).show(
                        ui.ctx(),
                        |ui| {
                            ui.set_width(280.0);

                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(format!(
                                                        "{} {}?",
                                                        self.i18n.get(LangKey::EditKey),
                                                        self.key_name(),
                                                    ))
                                                    .heading(),
                                                )
                                                .truncate(),
                                            );
                                        });
                                    },
                                );
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                "You have unsaved changes. What would you like to do?",
                                            );
                                        });
                                    },
                                );
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                if ui.button(self.i18n.get(LangKey::Cancel)).clicked() {
                                    self.show_unsaved_dialog = false;
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_enabled(
                                                self.has_unsaved_changes,
                                                egui::Button::new(
                                                    self.i18n.get(LangKey::Save),
                                                ),
                                            )
                                            .clicked()
                                        {
                                            self.save_changes(sender);
                                            self.show_unsaved_dialog = false;
                                            *open = false;
                                        }

                                        if ui.button("Don't save").clicked() {
                                            self.show_unsaved_dialog = false;
                                            *open = false;
                                        }
                                    },
                                );
                            });
                        },
                    );
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::KeyType;

    #[test]
    fn test_new_string_key() {
        let edit_key = EditKey::new(
            "mykey".to_string(),
            KeyType::String,
            vec!["hello world".to_string()],
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.key_name(), "mykey");
        assert!(matches!(edit_key.key_type, KeyType::String));
        assert_eq!(edit_key.data.len(), 1);
        assert_eq!(
            edit_key.data[0],
            ("".to_string(), "hello world".to_string())
        );
    }

    #[test]
    fn test_new_string_key_empty() {
        let edit_key = EditKey::new(
            "empty".to_string(),
            KeyType::String,
            vec![],
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.data.len(), 1);
        assert_eq!(edit_key.data[0], ("".to_string(), "".to_string()));
    }

    #[test]
    fn test_new_list_key() {
        let data = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let edit_key = EditKey::new(
            "mylist".to_string(),
            KeyType::List,
            data,
            Arc::new(Default::default()),
        );

        assert!(matches!(edit_key.key_type, KeyType::List));
        assert_eq!(edit_key.data.len(), 3);
        assert_eq!(edit_key.data[0], ("0".to_string(), "first".to_string()));
        assert_eq!(edit_key.data[1], ("1".to_string(), "second".to_string()));
        assert_eq!(edit_key.data[2], ("2".to_string(), "third".to_string()));
    }

    #[test]
    fn test_new_hash_key() {
        let data = vec![
            "field1".to_string(),
            "value1".to_string(),
            "field2".to_string(),
            "value2".to_string(),
        ];
        let edit_key = EditKey::new(
            "myhash".to_string(),
            KeyType::Hash,
            data,
            Arc::new(Default::default()),
        );

        assert!(matches!(edit_key.key_type, KeyType::Hash));
        assert_eq!(edit_key.data.len(), 2);
        assert_eq!(
            edit_key.data[0],
            ("field1".to_string(), "value1".to_string())
        );
        assert_eq!(
            edit_key.data[1],
            ("field2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_new_hash_key_odd_data() {
        let data = vec![
            "field1".to_string(),
            "value1".to_string(),
            "field2".to_string(),
        ];
        let edit_key = EditKey::new(
            "myhash".to_string(),
            KeyType::Hash,
            data,
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.data.len(), 2);
        assert_eq!(
            edit_key.data[0],
            ("field1".to_string(), "value1".to_string())
        );
        assert_eq!(edit_key.data[1], ("field2".to_string(), "".to_string()));
    }

    #[test]
    fn test_new_set_key() {
        let data = vec![
            "member1".to_string(),
            "member2".to_string(),
            "member3".to_string(),
        ];
        let edit_key = EditKey::new(
            "myset".to_string(),
            KeyType::Set,
            data,
            Arc::new(Default::default()),
        );

        assert!(matches!(edit_key.key_type, KeyType::Set));
        assert_eq!(edit_key.data.len(), 3);
        assert_eq!(edit_key.data[0], ("member1".to_string(), "".to_string()));
        assert_eq!(edit_key.data[1], ("member2".to_string(), "".to_string()));
        assert_eq!(edit_key.data[2], ("member3".to_string(), "".to_string()));
    }

    #[test]
    fn test_new_sorted_set_key() {
        let data = vec![
            "member1".to_string(),
            "1.5".to_string(),
            "member2".to_string(),
            "2.0".to_string(),
        ];
        let edit_key = EditKey::new(
            "myzset".to_string(),
            KeyType::SortedSet,
            data,
            Arc::new(Default::default()),
        );

        assert!(matches!(edit_key.key_type, KeyType::SortedSet));
        assert_eq!(edit_key.data.len(), 2);
        assert_eq!(edit_key.data[0], ("member1".to_string(), "1.5".to_string()));
        assert_eq!(edit_key.data[1], ("member2".to_string(), "2.0".to_string()));
    }

    #[test]
    fn test_row_deletion_tracking() {
        let mut edit_key = EditKey::new(
            "test".to_string(),
            KeyType::List,
            vec![
                "item0".to_string(),
                "item1".to_string(),
                "item2".to_string(),
            ],
            Arc::new(Default::default()),
        );

        edit_key.rows_to_delete.push(1);
        edit_key.rows_to_delete.push(2);

        assert_eq!(edit_key.rows_to_delete, vec![1, 2]);
        assert_eq!(edit_key.data.len(), 3);
        edit_key.delete_marked_rows();
        assert_eq!(edit_key.data.len(), 1);
    }

    #[test]
    fn test_focus_management() {
        let mut edit_key = EditKey::new(
            "test".to_string(),
            KeyType::Hash,
            vec!["field".to_string(), "value".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(edit_key.focused_cell, None);

        edit_key.focused_cell = Some((0, 1));
        assert_eq!(edit_key.focused_cell, Some((0, 1)));

        edit_key.focused_cell = None;
        assert_eq!(edit_key.focused_cell, None);
    }

    #[test]
    fn test_original_values_storage() {
        let mut edit_key = EditKey::new(
            "test".to_string(),
            KeyType::String,
            vec!["initial".to_string()],
            Arc::new(Default::default()),
        );

        let cell_id = (0, 1);
        edit_key
            .original_values
            .insert(cell_id, "original_value".to_string());

        assert_eq!(
            edit_key.original_values.get(&cell_id),
            Some(&"original_value".to_string())
        );

        edit_key.original_values.remove(&cell_id);
        assert!(!edit_key.original_values.contains_key(&cell_id));
    }

    #[test]
    fn test_data_structure_consistency() {
        let string_key = EditKey::new(
            "str".to_string(),
            KeyType::String,
            vec!["value".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(string_key.data.len(), 1);
        assert_eq!(string_key.data[0].0, "");

        let list_key = EditKey::new(
            "list".to_string(),
            KeyType::List,
            vec!["val".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(list_key.data[0].0, "0");
        assert_eq!(list_key.data[0].1, "val");

        let set_key = EditKey::new(
            "set".to_string(),
            KeyType::Set,
            vec!["member".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(set_key.data[0].0, "member");
        assert_eq!(set_key.data[0].1, "");

        let hash_key = EditKey::new(
            "hash".to_string(),
            KeyType::Hash,
            vec!["field".to_string(), "value".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(hash_key.data[0].0, "field");
        assert_eq!(hash_key.data[0].1, "value");

        let zset_key = EditKey::new(
            "zset".to_string(),
            KeyType::SortedSet,
            vec!["member".to_string(), "1.0".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(zset_key.data[0].0, "member");
        assert_eq!(zset_key.data[0].1, "1.0");
    }

    #[test]
    fn test_large_dataset_handling() {
        let large_data: Vec<String> = (0..100).map(|i| format!("item{i}")).collect();
        let edit_key = EditKey::new(
            "large_list".to_string(),
            KeyType::List,
            large_data,
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.data.len(), 100);
        assert_eq!(edit_key.data[99], ("99".to_string(), "item99".to_string()));
    }

    #[test]
    fn test_empty_data_handling() {
        let empty_string = EditKey::new(
            "empty".to_string(),
            KeyType::String,
            vec![],
            Arc::new(Default::default()),
        );
        assert_eq!(empty_string.data.len(), 1);
        assert_eq!(empty_string.data[0], ("".to_string(), "".to_string()));

        let empty_list = EditKey::new(
            "empty".to_string(),
            KeyType::List,
            vec![],
            Arc::new(Default::default()),
        );
        assert!(empty_list.data.is_empty());

        let empty_hash = EditKey::new(
            "empty".to_string(),
            KeyType::Hash,
            vec![],
            Arc::new(Default::default()),
        );
        assert!(empty_hash.data.is_empty());

        let empty_set = EditKey::new(
            "empty".to_string(),
            KeyType::Set,
            vec![],
            Arc::new(Default::default()),
        );
        assert!(empty_set.data.is_empty());

        let empty_zset = EditKey::new(
            "empty".to_string(),
            KeyType::SortedSet,
            vec![],
            Arc::new(Default::default()),
        );
        assert!(empty_zset.data.is_empty());
    }

    #[test]
    fn test_key_name_retrieval() {
        let edit_key = EditKey::new(
            "my:special:key".to_string(),
            KeyType::String,
            vec!["value".to_string()],
            Arc::new(Default::default()),
        );
        assert_eq!(edit_key.key_name(), "my:special:key");
    }

    #[test]
    fn test_state_initialization() {
        let edit_key = EditKey::new(
            "test".to_string(),
            KeyType::Hash,
            vec!["field".to_string(), "value".to_string()],
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.focused_cell, None);
        assert!(edit_key.original_values.is_empty());
        assert!(edit_key.rows_to_delete.is_empty());
    }

    #[test]
    fn test_add_form_initialization() {
        let edit_key = EditKey::new(
            "test".to_string(),
            KeyType::Hash,
            vec![],
            Arc::new(Default::default()),
        );

        assert_eq!(edit_key.new_field_name, "");
        assert_eq!(edit_key.new_field_value, "");
        assert!(!edit_key.show_add_form);
    }

    #[test]
    fn test_add_form_validation() {
        let mut edit_key = EditKey::new(
            "test".to_string(),
            KeyType::Hash,
            vec![],
            Arc::new(Default::default()),
        );

        edit_key.new_field_name = "".to_string();
        edit_key.new_field_value = "value".to_string();

        edit_key.new_field_name = "field".to_string();
        edit_key.new_field_value = "".to_string();

        edit_key.new_field_name = "field".to_string();
        edit_key.new_field_value = "value".to_string();
    }

    #[test]
    fn test_sorted_set_score_validation() {
        let mut edit_key = EditKey::new(
            "test".to_string(),
            KeyType::SortedSet,
            vec![],
            Arc::new(Default::default()),
        );

        edit_key.new_field_name = "member".to_string();
        edit_key.new_field_value = "invalid_score".to_string();

        edit_key.new_field_value = "1.5".to_string();
    }
}
