use crate::i18n::{I18N, LangKey};
use crate::state::{Message, RespCommand};
use crate::ui::widgets::popups::PopupUi;
use crate::utils::{KeyType, text_float_filter, text_float_filter_less_than_one};
use egui::{ScrollArea, Ui};
use std::string::String;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

const TRASH_ICON_SHOW_DELAY: u64 = 150;
const SCROLL_VIEW_HEIGHT: f32 = 300.0;

pub struct AddKey {
    key: String,
    ttl: String,
    selected_idx: usize,
    key_type: KeyType,
    key_form: KeyForm,
    push_to_head: bool,
    show_bloom_options: bool,
    bloom_filter_options: BloomFilterOptions,
}

#[derive(Default)]
struct KeyForm {
    col0: Vec<String>,
    col1: Vec<String>,
    row_states: Vec<Option<Instant>>,
    deletable: Vec<bool>,
}

#[derive(Default)]
struct BloomFilterOptions {
    error_rate: String,
    capacity: String,
    expansion: String,
    non_scaling: bool,
}

impl KeyForm {
    fn ui(&mut self, key_type: &KeyType, ui: &mut Ui) {
        if self.col0.is_empty() || self.col1.is_empty() {
            self.col0.clear();
            self.col1.clear();
            self.row_states.clear();
            self.deletable.clear();
            self.col0.push(String::new());
            self.col1.push(String::new());
            self.row_states.push(None);
            self.deletable.push(false);
        }

        let lact_col0_empty = self.col0.last().is_none_or(|v| v.is_empty());
        let last_col1_empty = self.col1.last().is_none_or(|v| v.is_empty());

        let old_len = self.col0.len();
        if !lact_col0_empty || !last_col1_empty {
            self.col0.push(String::new());
            self.col1.push(String::new());
            self.row_states.push(None);
            self.deletable.push(false);
        }
        let new_field_added = self.col0.len() > old_len;

        while self.row_states.len() < self.col0.len() {
            self.row_states.push(None);
            self.deletable.push(false);
        }

        let mut fields_to_remove = Vec::new();
        let indices: Vec<usize> = (0..self.col0.len()).collect();
        let now = Instant::now();

        if matches!(key_type, KeyType::String) {
            ui.add(
                egui::TextEdit::multiline(&mut self.col0[0])
                    .desired_width(ui.available_width())
                    .hint_text("Content"),
            );
        } else {
            for i in indices {
                ui.horizontal(|ui| {
                    let is_last = i == self.col0.len() - 1;
                    let is_only_field = self.col0.len() == 1;
                    let is_last_empty =
                        is_last && self.col0[i].is_empty() && self.col1[i].is_empty();

                    let mut col_has_focus: [bool; 2] = [false, false];

                    let double_layout = matches!(key_type, KeyType::Hash | KeyType::SortedSet);

                    ui.vertical(|ui| {
                        col_has_focus[if matches!(key_type, KeyType::Bloom) {
                            1
                        } else {
                            0
                        }] = ui
                            .add(
                                egui::TextEdit::singleline(if matches!(key_type, KeyType::Bloom) {
                                    &mut self.col1[i]
                                } else {
                                    &mut self.col0[i]
                                })
                                .desired_width(
                                    ui.available_width() * if double_layout { 0.3 } else { 0.92 },
                                )
                                .hint_text(
                                    if matches!(key_type, KeyType::List) {
                                        "Element"
                                    } else if matches!(key_type, KeyType::Set | KeyType::SortedSet)
                                    {
                                        "Member"
                                    } else if matches!(key_type, KeyType::Bloom) {
                                        "Item"
                                    } else {
                                        "Key"
                                    },
                                ),
                            )
                            .has_focus();
                    });

                    if double_layout {
                        ui.vertical(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.col1[i])
                                    .desired_width(ui.available_width() * 0.9)
                                    .hint_text(if matches!(key_type, KeyType::SortedSet) {
                                        "Score"
                                    } else {
                                        "Value"
                                    }),
                            );

                            col_has_focus[1] = response.has_focus();
                            if response.changed() && matches!(key_type, KeyType::SortedSet) {
                                self.col1[i] = text_float_filter(&self.col1[i])
                            }
                        });
                    }

                    let row_has_focus = col_has_focus[0] || col_has_focus[1];

                    if row_has_focus {
                        self.row_states[i] = None;
                    } else if self.row_states[i].is_none() {
                        self.row_states[i] = Some(now);
                    }

                    let show_delete = self.deletable[i]
                        || !is_only_field
                            && !is_last_empty
                            && !row_has_focus
                            && self.row_states[i].is_some_and(|lost_time| {
                                now.duration_since(lost_time)
                                    >= Duration::from_millis(TRASH_ICON_SHOW_DELAY)
                            });

                    if show_delete || self.deletable[i] {
                        self.deletable[i] = true;
                    }

                    if show_delete && ui.small_button("🗑").clicked() {
                        fields_to_remove.push(i);
                    }
                });
            }
        }

        for &i in fields_to_remove.iter().rev() {
            self.col0.remove(i);
            self.col1.remove(i);
            self.row_states.remove(i);
            self.deletable.remove(i);
        }

        if new_field_added {
            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
        }
    }
}

impl Default for AddKey {
    fn default() -> Self {
        Self {
            key: "".to_string(),
            ttl: "".to_string(),
            selected_idx: 0,
            key_type: Default::default(),
            key_form: Default::default(),
            push_to_head: false,
            show_bloom_options: false,
            bloom_filter_options: Default::default(),
        }
    }
}

impl AddKey {
    fn save(&mut self, sender: &Arc<Sender<Message>>) {
        let mut commands = Vec::new();
        let mut data: String = String::new();
        let col0_len = self.key_form.col0.len();
        let col1_len = self.key_form.col1.len();
        let col0_data = &mut self.key_form.col0[0..col0_len - 1];
        let col1_data = &mut self.key_form.col1[0..col1_len - 1];

        let quote_if_needed = |value: &str| -> String {
            if value.is_empty() || value.contains(' ') {
                format!("\"{value}\"")
            } else {
                value.to_string()
            }
        };

        for col0 in col0_data.iter_mut() {
            *col0 = quote_if_needed(col0);
        }
        for col1 in col1_data.iter_mut() {
            *col1 = quote_if_needed(col1);
        }

        let command_type = match self.key_type {
            KeyType::Hash | KeyType::SortedSet => {
                for (col0, col1) in col0_data.iter().zip(col1_data.iter()) {
                    if matches!(self.key_type, KeyType::SortedSet) {
                        data.push_str(col1);
                        data.push(' ');
                        data.push_str(col0);
                    } else {
                        data.push_str(col0);
                        data.push(' ');
                        data.push_str(col1);
                    }
                    data.push(' ');
                }
                match self.key_type {
                    KeyType::Hash => "HSET",
                    _ => "ZADD",
                }
            }
            KeyType::List | KeyType::Set => {
                for col0 in col0_data.iter() {
                    data.push_str(col0);
                    data.push(' ');
                }
                match self.key_type {
                    KeyType::List => {
                        if self.push_to_head {
                            "LPUSH"
                        } else {
                            "RPUSH"
                        }
                    }
                    _ => "SADD",
                }
            }
            KeyType::String => {
                if let Some(s) = self.key_form.col0.first() {
                    data = quote_if_needed(s);
                } else {
                    data = "\"\"".to_owned();
                }
                "SET"
            }
            KeyType::Bloom => {
                for col1 in col1_data.iter() {
                    data.push_str(col1);
                    data.push(' ');
                }
                if col1_data.len() <= 1 {
                    "BF.ADD"
                } else {
                    "BF.INSERT"
                }
            }
        };

        let key = quote_if_needed(&self.key);

        if self.show_bloom_options {
            let error_rate = self
                .bloom_filter_options
                .error_rate
                .parse::<f64>()
                .map(|rate| rate.clamp(0.0, 1.0))
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Invalid error rate '{}': {}. Using default 0.01",
                        self.bloom_filter_options.error_rate, e
                    );
                    0.01
                });
            let capacity = self
                .bloom_filter_options
                .capacity
                .parse::<usize>()
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Invalid capacity '{}': {}. Using default 1000",
                        self.bloom_filter_options.capacity, e
                    );
                    1000
                });
            let expansion = self
                .bloom_filter_options
                .expansion
                .parse::<usize>()
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Invalid expansion '{}': {}. Using default 3",
                        self.bloom_filter_options.expansion, e
                    );
                    3
                });
            let expansion_cmd_slice = if self.bloom_filter_options.non_scaling {
                "NONSCALING".to_string()
            } else if !self.bloom_filter_options.expansion.is_empty() {
                format!("EXPANSION {expansion}")
            } else {
                "".to_string()
            };
            commands.push(format!(
                "BF.RESERVE {key} {error_rate} {capacity} {expansion_cmd_slice}"
            ));
        }

        if !matches!(self.key_type, KeyType::Bloom) || !col1_data.is_empty() {
            commands.push(format!(
                "{command_type} {key}{}{data}",
                if !(matches!(self.key_type, KeyType::Bloom) && col1_data.len() > 1) {
                    " "
                } else {
                    " ITEMS "
                }
            ));
        }

        let ttl = self.ttl.parse::<i64>().unwrap_or(-1);
        if ttl > 0 {
            commands.push(format!("EXPIRE {key} {ttl}"));
        }
        sender
            .send(Message::ExecRespCommand(RespCommand::CommandRefresh(
                commands,
            )))
            .unwrap_or_else(|e| {
                eprintln!("Error sending message: {e}");
            });
    }
}

impl PopupUi for AddKey {
    fn ui(
        &mut self,
        ui: &mut Ui,
        sender: &Arc<Sender<Message>>,
        i18n: &Arc<I18N>,
        open: &mut bool,
    ) {
        ui.vertical(|ui| {
            ui.label("Key name*");
            ui.add(
                egui::TextEdit::singleline(&mut self.key)
                    .desired_width(ui.available_width())
                    .hint_text("My key"),
            );
            if matches!(self.key_type, KeyType::List) {
                ui.checkbox(&mut self.push_to_head, "Push to head");
            }
        });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Key type*");

                egui::ComboBox::new("key_type", "")
                    .width(ui.available_width() * 0.5)
                    .selected_text(KeyType::vector()[self.selected_idx].to_string())
                    .show_ui(ui, |ui| {
                        for (i, option) in KeyType::vector().iter().enumerate() {
                            if ui
                                .selectable_value(&mut self.selected_idx, i, option.to_string())
                                .changed()
                            {
                                self.key_type = KeyType::vector()[self.selected_idx];
                                if matches!(self.key_type, KeyType::SortedSet) {
                                    self.key_form.col1.iter_mut().for_each(|v| {
                                        *v = v.chars().filter(|c| c.is_ascii_digit()).collect()
                                    });
                                }
                            };
                        }
                    });
            });

            ui.vertical(|ui| {
                ui.label("TTL");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.ttl)
                            .desired_width(ui.available_width())
                            .hint_text("-1"),
                    )
                    .changed()
                {
                    self.ttl = self.ttl.chars().filter(|c| c.is_ascii_digit()).collect();
                }
            });
        });
        if matches!(self.key_type, KeyType::Bloom) {
            ui.checkbox(
                &mut self.show_bloom_options,
                "Set additional bloom filter options",
            );
            if self.show_bloom_options {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Error rate*");
                        if ui
                            .add(
                                egui::TextEdit::singleline(
                                    &mut self.bloom_filter_options.error_rate,
                                )
                                .desired_width(ui.available_width() * 0.5)
                                .hint_text("0.01"),
                            )
                            .changed()
                        {
                            self.bloom_filter_options.error_rate = text_float_filter_less_than_one(
                                &self.bloom_filter_options.error_rate.to_string(),
                            );
                        }
                    });

                    ui.vertical(|ui| {
                        ui.label("Capacity*");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut self.bloom_filter_options.capacity)
                                    .desired_width(ui.available_width())
                                    .hint_text("1000"),
                            )
                            .changed()
                        {
                            self.bloom_filter_options.capacity = self
                                .bloom_filter_options
                                .capacity
                                .chars()
                                .filter(|c| c.is_ascii_digit())
                                .collect();
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Expansion");
                        if ui
                            .add_enabled(
                                !self.bloom_filter_options.non_scaling,
                                egui::TextEdit::singleline(
                                    &mut self.bloom_filter_options.expansion,
                                )
                                .desired_width(ui.available_width() * 0.5)
                                .hint_text("3"),
                            )
                            .changed()
                        {
                            self.bloom_filter_options.expansion = self
                                .bloom_filter_options
                                .expansion
                                .chars()
                                .filter(|c| c.is_ascii_digit())
                                .collect();
                        }
                    });

                    ui.vertical(|ui| {
                        ui.add_space(16.0);
                        ui.checkbox(&mut self.bloom_filter_options.non_scaling, "Non-scaling");
                    });
                });
            }
        }

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        ScrollArea::vertical()
            .id_salt("add_key_form_scroll")
            .max_height(SCROLL_VIEW_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                self.key_form.ui(&self.key_type, ui);
            });
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if ui.button(i18n.get(LangKey::Cancel)).clicked() {
                *open = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.key.is_empty()
                            && (!self.show_bloom_options
                                || (!self.bloom_filter_options.error_rate.is_empty()
                                    && !self.bloom_filter_options.capacity.is_empty())),
                        egui::Button::new(i18n.get(LangKey::Save)),
                    )
                    .clicked()
                {
                    self.save(sender);
                    *open = false;
                }
            });
        });
    }
}
