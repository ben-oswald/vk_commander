use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::{AppState, ResultViewMode};
use crate::ui::Component;
use crate::utils::ValkeyClient;
use egui::{Button, Context, Key};
use egui_extras::{Column, TableBuilder};
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Default)]
pub struct WorkbenchWindow {
    resp_result: Arc<RwLock<String>>,
    resp_data: Arc<RwLock<Vec<String>>>,
    is_executing: Arc<RwLock<bool>>,
    result_display: String,
}

impl WorkbenchWindow {
    fn add_to_history(&self, state: &mut AppState, command: String) {
        if !command.trim().is_empty()
            && (state.workbench_state.command_history.is_empty()
                || state.workbench_state.command_history.last() != Some(&command))
        {
            state.workbench_state.command_history.push(command);

            if state.workbench_state.command_history.len() > 100 {
                state.workbench_state.command_history.remove(0);
            }
        }
        state.workbench_state.history_index = None;
        state.workbench_state.temp_command.clear();
    }

    fn navigate_history(&self, state: &mut AppState, direction: i32) {
        if state.workbench_state.command_history.is_empty() {
            return;
        }

        match state.workbench_state.history_index {
            None => {
                if direction < 0 && !state.workbench_state.command_history.is_empty() {
                    state.workbench_state.temp_command = state.workbench_state.resp_command.clone();
                    let last_index = state.workbench_state.command_history.len() - 1;
                    state.workbench_state.history_index = Some(last_index);
                    state.workbench_state.resp_command =
                        state.workbench_state.command_history[last_index].clone();
                }
            }
            Some(current_index) => {
                if direction < 0 && current_index > 0 {
                    let new_index = current_index - 1;
                    state.workbench_state.history_index = Some(new_index);
                    state.workbench_state.resp_command =
                        state.workbench_state.command_history[new_index].clone();
                } else if direction > 0 {
                    if current_index < state.workbench_state.command_history.len() - 1 {
                        let new_index = current_index + 1;
                        state.workbench_state.history_index = Some(new_index);
                        state.workbench_state.resp_command =
                            state.workbench_state.command_history[new_index].clone();
                    } else {
                        state.workbench_state.history_index = None;
                        state.workbench_state.resp_command =
                            state.workbench_state.temp_command.clone();
                    }
                }
            }
        }
    }
}

impl Component for WorkbenchWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        let sender = state.get_sender();
        if let Some(valkey_client) = state.valkey_client.clone() {
            egui::CentralPanel::default().show(ctx, |ui| {
                let button_height = 24.0;
                let is_executing = self.is_executing.read().is_ok_and(|guard| *guard);

                let suggestions = state
                    .command_registry
                    .get_suggestions(&state.workbench_state.resp_command);

                state.workbench_state.show_autocomplete = !suggestions.is_empty() && !is_executing;

                let mut text_edit_rect = egui::Rect::NOTHING;

                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{}:",
                                state.i18n().get(LangKey::RespCommand)
                            ))
                            .strong()
                            .size(14.0),
                        );
                    });
                    ui.add_space(4.0);
                });

                let hint_text = state.i18n().get(LangKey::RespCommand);
                let text_edit_id = ui.id().with("workbench_command_input");

                if !state.workbench_state.is_multiline {
                    ctx.input(|i| {
                        for event in &i.events {
                            if let egui::Event::Paste(text) = event
                                && text.contains('\n')
                            {
                                state.workbench_state.is_multiline = true;
                            }
                        }
                    });
                }

                if ctx.input(|i| i.key_pressed(Key::Enter) && i.modifiers.shift)
                    && !state.workbench_state.is_multiline
                {
                    state.workbench_state.is_multiline = true;
                }

                let text_edit_height = if state.workbench_state.is_multiline {
                    button_height * 10.0 // ~ 10 rows
                } else {
                    button_height
                };

                let text_edit_response = ui
                    .horizontal(|ui| {
                        ui.add_space(4.0);

                        let text_edit_response = if state.workbench_state.is_multiline {
                            egui::ScrollArea::vertical()
                                .min_scrolled_height(text_edit_height)
                                .id_salt("multiline_command_input_scroll")
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        [ui.available_width(), text_edit_height],
                                        egui::TextEdit::multiline(
                                            &mut state.workbench_state.resp_command,
                                        )
                                        .id(text_edit_id)
                                        .hint_text(hint_text)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_rows(10)
                                        .interactive(!is_executing),
                                    )
                                })
                                .inner
                        } else {
                            ui.add_sized(
                                [ui.available_width(), text_edit_height],
                                egui::TextEdit::singleline(&mut state.workbench_state.resp_command)
                                    .id(text_edit_id)
                                    .hint_text(hint_text)
                                    .font(egui::TextStyle::Monospace)
                                    .interactive(!is_executing),
                            )
                        };

                        text_edit_rect = text_edit_response.rect;

                        if text_edit_response.changed() {
                            state.workbench_state.autocomplete_selected_index = 0;

                            if !state.workbench_state.is_multiline
                                && state.workbench_state.resp_command.contains('\n')
                            {
                                state.workbench_state.is_multiline = true;
                            }

                            if state.workbench_state.is_multiline
                                && !state.workbench_state.resp_command.contains('\n')
                            {
                                state.workbench_state.is_multiline = false;
                            }
                        }

                        if let Some(cursor_pos) = state.workbench_state.set_cursor_pos {
                            if let Some(mut text_edit_state) =
                                egui::TextEdit::load_state(ui.ctx(), text_edit_id)
                            {
                                text_edit_state.cursor.set_char_range(Some(
                                    egui::text::CCursorRange::one(egui::text::CCursor::new(
                                        cursor_pos,
                                    )),
                                ));
                                text_edit_state.store(ui.ctx(), text_edit_id);
                            }
                            text_edit_response.request_focus();
                            state.workbench_state.set_cursor_pos = None;
                        }

                        if state.workbench_state.show_autocomplete
                            && !suggestions.is_empty()
                            && text_edit_response.lost_focus()
                            && ctx.input(|i| i.key_pressed(Key::Enter))
                        {
                            let selected_suggestion =
                                &suggestions[state.workbench_state.autocomplete_selected_index];
                            state.workbench_state.resp_command =
                                format!("{} ", selected_suggestion.full_name);
                            state.workbench_state.show_autocomplete = false;
                            state.workbench_state.autocomplete_selected_index = 0;
                            text_edit_response.request_focus();

                            if let Some(mut text_edit_state) =
                                egui::TextEdit::load_state(ui.ctx(), text_edit_id)
                            {
                                let cursor_pos = state.workbench_state.resp_command.len();
                                text_edit_state.cursor.set_char_range(Some(
                                    egui::text::CCursorRange::one(egui::text::CCursor::new(
                                        cursor_pos,
                                    )),
                                ));
                                text_edit_state.store(ui.ctx(), text_edit_id);
                            }
                        }

                        if text_edit_response.has_focus() {
                            if state.workbench_state.show_autocomplete && !suggestions.is_empty() {
                                if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                                    state.workbench_state.autocomplete_selected_index =
                                        (state.workbench_state.autocomplete_selected_index + 1)
                                            .min(suggestions.len() - 1);
                                } else if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                                    state.workbench_state.autocomplete_selected_index = state
                                        .workbench_state
                                        .autocomplete_selected_index
                                        .saturating_sub(1);
                                } else if ctx.input(|i| i.key_pressed(Key::Tab)) {
                                    state.workbench_state.autocomplete_selected_index =
                                        (state.workbench_state.autocomplete_selected_index + 1)
                                            % suggestions.len();
                                } else if ctx.input(|i| i.key_pressed(Key::Escape)) {
                                    state.workbench_state.show_autocomplete = false;
                                }
                            } else if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                                self.navigate_history(state, -1);
                            } else if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                                self.navigate_history(state, 1);
                            }
                        }

                        text_edit_response
                    })
                    .inner;

                let enter_pressed = if state.workbench_state.is_multiline {
                    text_edit_response.has_focus()
                        && ctx.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift)
                        && !state.workbench_state.show_autocomplete
                } else {
                    text_edit_response.lost_focus()
                        && ctx.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift)
                        && !state.workbench_state.show_autocomplete
                };

                ui.add_space(4.0);

                let execute_command = ui
                    .horizontal(|ui| {
                        ui.add_space(4.0);

                        let button_text = if is_executing {
                            format!("{}...", state.i18n().get(LangKey::Executing))
                        } else {
                            state.i18n().get(LangKey::Exec)
                        };
                        let button_enabled =
                            !is_executing && !state.workbench_state.resp_command.trim().is_empty();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_sized([100.0, button_height], Button::new(button_text))
                                .clicked()
                                && button_enabled
                        })
                        .inner
                    })
                    .inner;

                if execute_command
                    || (enter_pressed
                        && !is_executing
                        && !state.workbench_state.resp_command.trim().is_empty())
                {
                    let command = state.workbench_state.resp_command.trim().to_string();
                    self.add_to_history(state, command.clone());

                    let res_result = self.resp_result.clone();
                    let res_data = self.resp_data.clone();
                    let is_executing_clone = self.is_executing.clone();
                    let ctx_clone = ctx.clone();

                    if let Ok(mut guard) = is_executing_clone.write() {
                        *guard = true;
                    }
                    if let Ok(mut guard) = res_result.write() {
                        *guard = state.i18n().get(LangKey::Executing);
                    }
                    ctx.request_repaint();

                    let i18n = state.i18n();
                    thread::spawn(move || {
                        let commands: Vec<&str> = command
                            .lines()
                            .map(|line| line.trim())
                            .filter(|line| !line.is_empty() && !line.starts_with('#'))
                            .collect();

                        let mut all_results = Vec::new();
                        let mut all_responses = Vec::new();

                        for (idx, cmd) in commands.iter().enumerate() {
                            let result = match valkey_client.exec(cmd) {
                                Ok(response) => {
                                    all_responses.extend(response.clone());

                                    if response.is_empty() {
                                        format!(
                                            "{}. {}: {}",
                                            idx + 1,
                                            cmd,
                                            i18n.get(LangKey::NoResponse)
                                        )
                                    } else if response.len() == 1 {
                                        format!("{}. {}: {}", idx + 1, cmd, response[0])
                                    } else {
                                        let items = response
                                            .iter()
                                            .enumerate()
                                            .map(|(i, item)| format!("   {}) \"{}\"", i + 1, item))
                                            .collect::<Vec<String>>()
                                            .join("\n");
                                        format!("{}. {}:\n{}", idx + 1, cmd, items)
                                    }
                                }
                                Err(e) => {
                                    e.show_error_dialog(sender.clone());
                                    format!(
                                        "{}. {}: {}",
                                        idx + 1,
                                        cmd,
                                        i18n.get(LangKey::AnErrorOccurred)
                                    )
                                }
                            };
                            all_results.push(result);
                        }

                        if let Ok(mut guard) = res_data.write() {
                            *guard = all_responses;
                        }

                        let final_result = if all_results.is_empty() {
                            i18n.get(LangKey::NoResponse)
                        } else {
                            all_results.join("\n")
                        };

                        if let Ok(mut guard) = res_result.write() {
                            *guard = final_result;
                        }
                        if let Ok(mut guard) = is_executing_clone.write() {
                            *guard = false;
                        }
                        ctx_clone.request_repaint();
                    });
                }

                if state.workbench_state.show_autocomplete && !suggestions.is_empty() {
                    let input_len = state.workbench_state.resp_command.len();

                    let popup_pos =
                        egui::pos2(text_edit_rect.left(), text_edit_rect.bottom() + 2.0);

                    egui::Area::new(egui::Id::new("workbench_autocomplete_popup"))
                        .fixed_pos(popup_pos)
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style())
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.set_min_width(250.0);
                                    ui.set_max_width(500.0);

                                    egui::ScrollArea::vertical()
                                        .id_salt("workbench_autocomplete_scroll")
                                        .max_height(200.0)
                                        .show(ui, |ui| {
                                            ui.spacing_mut().item_spacing.y = 2.0;

                                            for (idx, suggestion) in suggestions.iter().enumerate()
                                            {
                                                let is_selected = idx
                                                    == state
                                                        .workbench_state
                                                        .autocomplete_selected_index;

                                                let (row_rect, row_response) = ui
                                                    .allocate_exact_size(
                                                        egui::vec2(
                                                            ui.available_width(),
                                                            ui.spacing().interact_size.y,
                                                        ),
                                                        egui::Sense::click(),
                                                    );

                                                if is_selected {
                                                    ui.painter().rect_filled(
                                                        row_rect,
                                                        2.0,
                                                        ui.visuals().selection.bg_fill,
                                                    );
                                                }

                                                let mut child_ui = ui.new_child(
                                                    egui::UiBuilder::new().max_rect(row_rect),
                                                );
                                                child_ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 0.0;

                                                    let typed_part = &suggestion.full_name
                                                        [..input_len
                                                            .min(suggestion.full_name.len())];
                                                    let remaining_part = &suggestion.full_name
                                                        [input_len
                                                            .min(suggestion.full_name.len())..];

                                                    let typed_label =
                                                        egui::RichText::new(typed_part).strong();
                                                    ui.label(typed_label);

                                                    ui.label(remaining_part);

                                                    if !suggestion.arguments_desc.is_empty() {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                " {}",
                                                                &suggestion.arguments_desc
                                                            ))
                                                            .weak(),
                                                        );
                                                    }
                                                });

                                                if row_response.clicked() {
                                                    state.workbench_state.resp_command =
                                                        format!("{} ", suggestion.full_name);
                                                    state.workbench_state.show_autocomplete = false;
                                                    state
                                                        .workbench_state
                                                        .autocomplete_selected_index = 0;
                                                }

                                                if row_response.hovered()
                                                    && !suggestion.summary.is_empty()
                                                {
                                                    row_response.on_hover_text(&suggestion.summary);
                                                }
                                            }
                                        });
                                });
                        });
                }

                ui.add_space(8.0);

                if let Ok(guard) = self.resp_data.read() {
                    state.workbench_state.result_data = guard.clone();
                }

                let available_height = ui.available_height();
                let results_height = if !state.workbench_state.command_history.is_empty() {
                    available_height * 0.75
                } else {
                    available_height
                };
                let history_height = available_height * 0.25;

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("{}:", state.i18n().get(LangKey::Result)))
                                .strong()
                                .size(14.0),
                        );

                        if state.workbench_state.result_data.len() > 1 {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut state.workbench_state.view_mode,
                                    ResultViewMode::Text,
                                    "📄 Text",
                                );
                                ui.selectable_value(
                                    &mut state.workbench_state.view_mode,
                                    ResultViewMode::Table,
                                    "📊 Table",
                                );
                            });
                        }

                        let result_text = self
                            .resp_result
                            .read()
                            .map_or(String::new(), |guard| guard.clone());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !result_text.is_empty() && !is_executing,
                                    Button::new(format!("📋 {}", state.i18n().get(LangKey::Copy))),
                                )
                                .clicked()
                            {
                                ctx.copy_text(result_text.clone());
                            }
                        });
                    });

                    ui.add_space(6.0);

                    self.result_display = self
                        .resp_result
                        .read()
                        .map_or(String::new(), |guard| guard.clone());

                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .corner_radius(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            if state.workbench_state.view_mode == ResultViewMode::Table
                                && state.workbench_state.result_data.len() > 1
                            {
                                egui::ScrollArea::vertical()
                                    .id_salt("workbench_table_scroll")
                                    .max_height(results_height - 60.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let table_builder = TableBuilder::new(ui)
                                            .column(
                                                Column::initial(60.0)
                                                    .range(40.0..=100.0)
                                                    .resizable(true),
                                            )
                                            .column(Column::remainder().at_least(200.0))
                                            .cell_layout(egui::Layout::left_to_right(
                                                egui::Align::Center,
                                            ))
                                            .striped(true);

                                        table_builder
                                            .header(20.0, |mut header| {
                                                header.col(|ui| {
                                                    ui.strong("Index");
                                                });
                                                header.col(|ui| {
                                                    ui.strong("Value");
                                                });
                                            })
                                            .body(|mut body| {
                                                for (index, item) in state
                                                    .workbench_state
                                                    .result_data
                                                    .iter()
                                                    .enumerate()
                                                {
                                                    body.row(18.0, |mut row| {
                                                        row.col(|ui| {
                                                            ui.label((index + 1).to_string());
                                                        });
                                                        row.col(|ui| {
                                                            ui.label(item);
                                                        });
                                                    });
                                                }
                                            });
                                    });
                            } else {
                                egui::ScrollArea::vertical()
                                    .id_salt("workbench_result_scroll")
                                    .max_height(results_height - 60.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.style_mut().override_font_id =
                                            Some(egui::FontId::monospace(13.0));
                                        ui.add(
                                            egui::Label::new(&self.result_display).selectable(true),
                                        );
                                    });
                            }
                        });
                });

                ui.add_space(8.0);

                if !state.workbench_state.command_history.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{}:",
                                state.i18n().get(LangKey::CommandHistory)
                            ))
                            .strong()
                            .size(14.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "({} commands)",
                                state.workbench_state.command_history.len()
                            ))
                            .weak()
                            .size(12.0),
                        );
                    });

                    ui.add_space(4.0);

                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .corner_radius(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("command_history_scroll")
                                .max_height(history_height - 60.0)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    let history_len = state.workbench_state.command_history.len();
                                    for (idx, cmd) in state
                                        .workbench_state
                                        .command_history
                                        .iter()
                                        .enumerate()
                                        .rev()
                                    {
                                        let history_idx = idx;
                                        let display_num = history_len - idx;

                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}.", display_num))
                                                    .weak()
                                                    .size(12.0),
                                            );

                                            let button_response = ui.add(
                                                egui::Button::new(
                                                    egui::RichText::new(cmd).monospace().size(13.0),
                                                )
                                                .frame(false)
                                                .wrap_mode(egui::TextWrapMode::Truncate),
                                            );

                                            if button_response.clicked() {
                                                state.workbench_state.resp_command = cmd.clone();
                                                state.workbench_state.history_index =
                                                    Some(history_idx);
                                            }

                                            if button_response.hovered() {
                                                button_response.on_hover_text(cmd);
                                            }
                                        });
                                    }
                                });
                        });

                    ui.add_space(8.0);
                }
            });
        }
        Ok(())
    }

    fn refresh(&mut self, _: &Arc<ValkeyClient>) {}
}
