use crate::errors::Error;
use crate::state::AppState;
use crate::state::Event::SetMainWindow;
use crate::state::{MainWindow, Message};
use crate::ui::Component;
use crate::utils::{ValkeyClient, get_commands_dir};
use egui::{Context, ScrollArea, TextEdit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandArgument {
    name: String,
    #[serde(rename = "type")]
    arg_type: String,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    arguments: Vec<CommandArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyRange {
    lastkey: i32,
    step: i32,
    limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FindKeys {
    range: KeyRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeySpec {
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    begin_search: Option<serde_json::Value>,
    #[serde(default)]
    find_keys: Option<FindKeys>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandInfo {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    complexity: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    since: String,
    #[serde(default)]
    arity: i32,
    #[serde(default)]
    function: String,
    #[serde(default)]
    container: String,
    #[serde(default)]
    command_flags: Vec<String>,
    #[serde(default)]
    acl_categories: Vec<String>,
    #[serde(default)]
    key_specs: Vec<KeySpec>,
    #[serde(default)]
    history: Vec<String>,
    #[serde(default)]
    arguments: Vec<CommandArgument>,
    #[serde(default)]
    reply_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandDoc {
    name: String,
    info: CommandInfo,
}

pub struct DocumentationWindow {
    commands: Vec<CommandDoc>,
    search_query: String,
    selected_command: Option<String>,
    loaded: bool,
}

impl Default for DocumentationWindow {
    fn default() -> Self {
        let mut window = Self {
            commands: Vec::new(),
            search_query: String::new(),
            selected_command: None,
            loaded: false,
        };
        window.load_commands();
        window
    }
}

impl DocumentationWindow {
    fn load_commands(&mut self) {
        if self.loaded {
            return;
        }

        let commands_dir = get_commands_dir();
        if !commands_dir.exists() {
            eprintln!("Commands directory not found");
            self.loaded = true;
            return;
        }

        let mut commands = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&commands_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                    && let Some(ext) = entry.path().extension()
                    && ext == "json"
                    && let Ok(content) = std::fs::read_to_string(entry.path())
                    && let Some(cmd) = Self::parse_command_json(&content)
                {
                    commands.push(cmd.clone());
                }
            }
        }

        commands.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.commands = commands;
        self.loaded = true;
    }

    fn parse_command_json(content: &str) -> Option<CommandDoc> {
        let json: serde_json::Value = serde_json::from_str(content).ok()?;
        let obj = json.as_object()?;

        let (base_name, info_value) = obj.iter().next()?;

        let info: CommandInfo = serde_json::from_value(info_value.clone()).ok()?;

        let full_name = if !info.container.is_empty() {
            format!("{} {}", info.container, base_name)
        } else {
            base_name.clone()
        };

        Some(CommandDoc {
            name: full_name,
            info,
        })
    }

    fn filter_commands(&self) -> Vec<CommandDoc> {
        if self.search_query.is_empty() {
            self.commands.clone()
        } else {
            let query = self.search_query.to_lowercase();
            self.commands
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().contains(&query)
                        || cmd.info.summary.to_lowercase().contains(&query)
                        || cmd.info.group.to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        }
    }

    fn group_commands(&self, commands: &[CommandDoc]) -> BTreeMap<String, Vec<CommandDoc>> {
        let mut grouped: BTreeMap<String, Vec<CommandDoc>> = BTreeMap::new();

        for cmd in commands {
            let group_name = if cmd.info.group.is_empty() {
                "other".to_string()
            } else {
                cmd.info.group.clone()
            };

            grouped.entry(group_name).or_default().push(cmd.clone());
        }

        for commands in grouped.values_mut() {
            commands.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        grouped
    }

    fn render_arguments(ui: &mut egui::Ui, arguments: &[CommandArgument], indent_level: usize) {
        for arg in arguments {
            let indent = "        ".repeat(indent_level);
            ui.horizontal(|ui| {
                ui.label(format!("{}<{}>", indent, arg.name));
                ui.weak(format!("({})", arg.arg_type));
            });

            if !arg.arguments.is_empty() {
                Self::render_arguments(ui, &arg.arguments, indent_level + 1);
            }
        }
    }

    fn render_reply_schema(ui: &mut egui::Ui, schema: &serde_json::Value, indent_level: usize) {
        let indent = "  ".repeat(indent_level);

        if let Some(obj) = schema.as_object() {
            if let Some(one_of) = obj.get("oneOf") {
                if let Some(variants) = one_of.as_array() {
                    ui.label(format!("{}One of:", indent));
                    for (idx, variant) in variants.iter().enumerate() {
                        ui.label(format!("{}  Option {}:", indent, idx + 1));
                        Self::render_reply_schema(ui, variant, indent_level + 2);
                    }
                }
                return;
            }

            if let Some(any_of) = obj.get("anyOf") {
                if let Some(variants) = any_of.as_array() {
                    ui.label(format!("{}Any of:", indent));
                    for (idx, variant) in variants.iter().enumerate() {
                        ui.label(format!("{}  Option {}:", indent, idx + 1));
                        Self::render_reply_schema(ui, variant, indent_level + 2);
                    }
                }
                return;
            }

            if let Some(const_val) = obj.get("const") {
                ui.horizontal(|ui| {
                    ui.label(format!("{}Constant:", indent));
                    ui.weak(format!("{}", const_val));
                });
            }

            if let Some(type_val) = obj.get("type")
                && let Some(type_str) = type_val.as_str()
            {
                ui.horizontal(|ui| {
                    ui.label(format!("{}Type:", indent));
                    ui.weak(type_str);
                });
            }

            if let Some(desc) = obj.get("description")
                && let Some(desc_str) = desc.as_str()
            {
                ui.horizontal(|ui| {
                    ui.label(format!("{}Description:", indent));
                    ui.weak(desc_str);
                });
            }

            if let Some(items) = obj.get("items") {
                ui.label(format!("{}Items:", indent));
                Self::render_reply_schema(ui, items, indent_level + 1);
            }

            if let Some(properties) = obj.get("properties")
                && let Some(props_obj) = properties.as_object()
            {
                ui.label(format!("{}Properties:", indent));
                for (prop_name, prop_schema) in props_obj {
                    ui.label(format!("{}  {}:", indent, prop_name));
                    Self::render_reply_schema(ui, prop_schema, indent_level + 2);
                }
            }
        }
    }

    fn open_command_in_workbench(state: &mut AppState, command_name: &str) {
        state.workbench_state.resp_command = format!("{} ", command_name);
        state.workbench_state.set_cursor_pos = Some(state.workbench_state.resp_command.len());

        if let Ok(mut current_window) = state.ui_panels.current_window.write() {
            *current_window = Some(MainWindow::Workbench);
        }

        let _ = state
            .get_sender()
            .send(Message::Event(Arc::from(SetMainWindow(MainWindow::Workbench))));
    }

    fn command_entry(
        &mut self,
        ui: &mut egui::Ui,
        display_text: String,
        command_full_name: &str,
        state: &mut AppState,
    ) {
        let is_selected = self.selected_command.as_deref() == Some(command_full_name);
        let response = ui.selectable_label(is_selected, display_text);

        if response.clicked() {
            self.selected_command = Some(command_full_name.to_string());
        }

        if response.double_clicked() {
            Self::open_command_in_workbench(state, command_full_name);
        }
    }

    fn labeled_row(ui: &mut egui::Ui, title: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.label(value);
        });
    }

    fn labeled_row_if_not_empty(ui: &mut egui::Ui, title: &str, value: &str) {
        if !value.is_empty() {
            Self::labeled_row(ui, title, value);
        }
    }
}

impl Component for DocumentationWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Valkey Command Documentation");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.add(
                    TextEdit::singleline(&mut self.search_query)
                        .desired_width(300.0)
                        .hint_text("Filter by command name, summary, or group..."),
                );
                if response.changed() {
                    self.selected_command = None;
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);

            let filtered_commands = self.filter_commands();
            let grouped_commands = self.group_commands(&filtered_commands);

            let available_height = ui.available_height();

            ui.horizontal(|ui| {
                ui.set_height(available_height);

                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    ui.strong(format!("Commands ({})", filtered_commands.len()));
                    ui.separator();

                    ScrollArea::vertical()
                        .id_salt("command_list_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for (group_name, commands) in &grouped_commands {
                                ui.add_space(5.0);
                                egui::CollapsingHeader::new(format!("📁 {}", group_name))
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        let mut by_container: BTreeMap<String, Vec<CommandDoc>> = BTreeMap::new();
                                        let mut parents: BTreeMap<String, CommandDoc> = BTreeMap::new();

                                        for cmd in commands {
                                            if cmd.info.container.is_empty() {
                                                parents.insert(cmd.name.clone(), cmd.clone());
                                            } else {
                                                by_container
                                                    .entry(cmd.info.container.clone())
                                                    .or_default()
                                                    .push(cmd.clone());
                                            }
                                        }

                                        for children in by_container.values_mut() {
                                            children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                                        }

                                        let mut container_keys: Vec<String> = by_container.keys().cloned().collect();
                                        container_keys.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

                                        for container in container_keys {
                                            egui::CollapsingHeader::new(format!("📦 {}", container))
                                                .default_open(false)
                                                .show(ui, |ui| {
                                                    if let Some(parent_cmd) = parents.get(&container).cloned() {
                                                        self.command_entry(
                                                            ui,
                                                            format!("  {}", &parent_cmd.name),
                                                            &parent_cmd.name,
                                                            state,
                                                        );
                                                    } else {
                                                        ui.weak(format!("  {}", container));
                                                    }

                                                    if let Some(children) = by_container.get(&container) {
                                                        for child in children {
                                                            let prefix = format!("{} ", &container);
                                                            let display = child.name.strip_prefix(&prefix).unwrap_or(&child.name);

                                                            self.command_entry(
                                                                ui,
                                                                format!("      {}", display),
                                                                &child.name,
                                                                state,
                                                            );
                                                        }
                                                    }
                                                });
                                        }

                                        let mut standalone: Vec<&CommandDoc> = Vec::new();
                                        for cmd in commands {
                                            if cmd.info.container.is_empty() && !by_container.contains_key(&cmd.name) {
                                                standalone.push(cmd);
                                            }
                                        }
                                        standalone.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                                        for cmd in standalone {
                                            self.command_entry(
                                                ui,
                                                format!("  {}", &cmd.name),
                                                &cmd.name,
                                                state,
                                            );
                                        }
                                    });
                            }
                        });
                });

                ui.separator();

                ui.vertical(|ui| {
                    ScrollArea::vertical()
                        .id_salt("command_details_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if let Some(selected) = &self.selected_command {
                                if let Some(cmd) =
                                    self.commands.iter().find(|c| &c.name == selected)
                                {
                                    ui.heading(&cmd.name);
                                    ui.add_space(10.0);

                                    ui.strong("Summary:");
                                    ui.label(&cmd.info.summary);
                                    ui.add_space(10.0);

                                    Self::labeled_row_if_not_empty(ui, "Group:", &cmd.info.group);
                                    Self::labeled_row_if_not_empty(ui, "Since:", &cmd.info.since);
                                    Self::labeled_row_if_not_empty(ui, "Complexity:", &cmd.info.complexity);
                                    if cmd.info.arity != 0 {
                                        Self::labeled_row(ui, "Arity:", &format!("{}", cmd.info.arity));
                                    }
                                    Self::labeled_row_if_not_empty(ui, "Function:", &cmd.info.function);
                                    if !cmd.info.command_flags.is_empty() {
                                        Self::labeled_row(ui, "Command Flags:", &cmd.info.command_flags.join(", "));
                                    }
                                    if !cmd.info.acl_categories.is_empty() {
                                        Self::labeled_row(ui, "ACL Categories:", &cmd.info.acl_categories.join(", "));
                                    }

                                    if !cmd.info.key_specs.is_empty() {
                                        ui.add_space(10.0);
                                        ui.strong("Key Specs:");
                                        ui.add_space(5.0);

                                        for (idx, spec) in cmd.info.key_specs.iter().enumerate() {
                                            if idx > 0 {
                                                ui.add_space(5.0);
                                            }

                                            ui.horizontal(|ui| {
                                                ui.label("  • Flags:".to_string());
                                                if !spec.flags.is_empty() {
                                                    ui.weak(spec.flags.join(", "));
                                                } else {
                                                    ui.weak("(none)");
                                                }
                                            });

                                            if let Some(ref begin_search) = spec.begin_search {
                                                ui.horizontal(|ui| {
                                                    ui.label("    Begin search:".to_string());
                                                    ui.weak(format!("{}", begin_search));
                                                });
                                            }

                                            if let Some(ref find_keys) = spec.find_keys {
                                                ui.horizontal(|ui| {
                                                    ui.label("    Key range:".to_string());
                                                    ui.weak(format!(
                                                        "lastkey={}, step={}, limit={}",
                                                        find_keys.range.lastkey,
                                                        find_keys.range.step,
                                                        find_keys.range.limit
                                                    ));
                                                });
                                            }
                                        }
                                    }

                                    if !cmd.info.history.is_empty() {
                                        ui.add_space(10.0);
                                        ui.strong("History:");
                                        ui.add_space(5.0);
                                        for entry in &cmd.info.history {
                                            ui.label(format!("  • {}", entry));
                                        }
                                    }

                                    if !cmd.info.arguments.is_empty() {
                                        ui.add_space(10.0);
                                        ui.strong("Arguments:");
                                        ui.add_space(5.0);

                                        Self::render_arguments(ui, &cmd.info.arguments, 0);
                                    }

                                    if let Some(ref schema) = cmd.info.reply_schema {
                                        ui.add_space(10.0);
                                        ui.strong("Reply Schema:");
                                        ui.add_space(5.0);
                                        Self::render_reply_schema(ui, schema, 0);
                                    }
                                } else {
                                    ui.label("Command not found");
                                }
                            } else {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.heading("Select a command to view details");
                                    ui.add_space(10.0);
                                    ui.label("Search or browse the command list on the left");
                                });
                            }
                        });
                });
            });
        });
        Ok(())
    }

    fn refresh(&mut self, _client: &Arc<ValkeyClient>) {
        unimplemented!()
    }
}
