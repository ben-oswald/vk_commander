use crate::errors::Error;
use crate::state::AppState;
use crate::ui::Component;
use crate::utils::{ValkeyClient, ValkeyUrl};
use crate::state::{Event, Message, BannerParams, BannerKind};
use csv::ReaderBuilder;
use egui::Context;
use std::sync::{Arc, RwLock};
use std::fs;
use egui_file_dialog::FileDialog;

struct LogEntry {
    message: String,
    is_error: bool,
}

pub struct ImportWindow {
    selected_alias: String,
    file_path: String,
    target_db: usize,
    overwrite: bool,
    is_importing: Arc<RwLock<bool>>,
    progress: Arc<RwLock<f64>>,
    logs: Arc<RwLock<Vec<LogEntry>>>,
    file_dialog: FileDialog,
}

impl Default for ImportWindow {
    fn default() -> Self {
        Self {
            selected_alias: String::new(),
            file_path: String::new(),
            target_db: 0,
            overwrite: true,
            is_importing: Arc::new(RwLock::new(false)),
            progress: Arc::new(RwLock::new(0.0)),
            logs: Arc::new(RwLock::new(Vec::new())),
            file_dialog: FileDialog::default(),
        }
    }
}

impl ImportWindow {
    fn commands_from_content(content: &str, file_path: &str, overwrite: bool) -> Result<Vec<String>, String> {
        if file_path.to_lowercase().ends_with(".csv") {
            Self::commands_from_csv(content, overwrite)
        } else {
            Ok(Self::commands_from_lines(content, overwrite))
        }
    }

    fn commands_from_lines(content: &str, overwrite: bool) -> Vec<String> {
        content
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty() {
                    return None;
                }
                let mut tokens = Self::tokenize_line(l);
                Self::apply_overwrite_policy_to_tokens(&mut tokens, overwrite);
                Some(Self::inline_command_from_tokens(&tokens))
            })
            .collect()
    }

    fn commands_from_csv(content: &str, overwrite: bool) -> Result<Vec<String>, String> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(content.as_bytes());

        let mut commands = Vec::new();

        for record in reader.records() {
            let record = record.map_err(|e| format!("CSV parse error: {}", e))?;
            let mut fields: Vec<String> = record
                .iter()
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect();

            if fields.is_empty() {
                continue;
            }

            if Self::looks_like_command(&fields[0]) {
                Self::apply_overwrite_policy_to_tokens(&mut fields, overwrite);
                commands.push(Self::inline_command_from_tokens(&fields));
            } else {
                let write_cmd = if overwrite { "SET" } else { "SETNX" };
                for key in fields {
                    commands.push(format!("{} {} \"\"", write_cmd, Self::quote_if_needed(&key)));
                }
            }
        }
        Ok(commands)
    }

    fn apply_overwrite_policy_to_tokens(tokens: &mut Vec<String>, overwrite: bool) {
        if overwrite || tokens.is_empty() {
            return;
        }

        let cmd_upper = tokens[0].to_ascii_uppercase();
        match cmd_upper.as_str() {
            "SET" => {
                if !tokens
                    .iter()
                    .any(|t| matches!(t.to_ascii_uppercase().as_str(), "NX" | "XX"))
                {
                    tokens.push("NX".to_string());
                }
            }
            "MSET" => tokens[0] = "MSETNX".to_string(),
            "SETEX" | "PSETEX" if tokens.len() >= 4 => {
                let (key, val, arg) = (tokens[1].clone(), tokens[3].clone(), tokens[2].clone());
                let mode = if cmd_upper == "SETEX" { "EX" } else { "PX" };
                *tokens = vec!["SET".into(), key, val, mode.into(), arg, "NX".into()];
            }
            _ => {}
        }
    }

    fn tokenize_line(line: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let (mut in_quotes, mut escaped, mut has_token) = (false, false, false);

        for c in line.chars() {
            if escaped {
                current.push(c);
                escaped = false;
                has_token = true;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_quotes = !in_quotes;
                has_token = true;
            } else if c.is_whitespace() && !in_quotes {
                if has_token {
                    tokens.push(std::mem::take(&mut current));
                    has_token = false;
                }
            } else {
                current.push(c);
                has_token = true;
            }
        }
        if has_token {
            tokens.push(current);
        }
        tokens
    }

    fn looks_like_command(token: &str) -> bool {
        const KNOWN: &[&str] = &[
            "SET", "SETNX", "SETEX", "PSETEX", "MSET", "MSETNX", "LPUSH", "RPUSH", "SADD", "ZADD", "HSET",
            "HMSET", "INCR", "DECR", "DEL", "GET", "EXPIRE", "PEXPIRE", "RESTORE", "PING", "EVAL", "KEYS",
        ];
        KNOWN.contains(&token.to_ascii_uppercase().as_str())
    }

    fn inline_command_from_tokens(tokens: &[String]) -> String {
        tokens
            .iter()
            .map(|t| Self::quote_if_needed(t))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn process_import_response(cmd: &str, resp: &str) -> Option<LogEntry> {
        if resp == "OK" {
            return None;
        }

        let tokens = Self::tokenize_line(cmd);
        let cmd_upper = tokens
            .first()
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        let is_nx = tokens.iter().any(|t| t.to_ascii_uppercase() == "NX")
            || matches!(cmd_upper.as_str(), "SETNX" | "MSETNX");

        if is_nx && (resp.is_empty() || resp == "0") {
            let key_msg = match cmd_upper.as_str() {
                "MSETNX" => "One or more keys".to_string(),
                _ if tokens.len() > 1 => format!("Key `{}`", tokens[1]),
                _ => "Key".to_string(),
            };
            return Some(LogEntry {
                message: format!("{} already exists, skipping", key_msg),
                is_error: false,
            });
        }

        if resp.parse::<i64>().is_ok() {
            return None;
        }

        Some(LogEntry {
            message: format!("Response for '{}' : {}", cmd, resp),
            is_error: true,
        })
    }

    fn quote_if_needed(token: &str) -> String {
        if token.is_empty() {
            "\"\"".to_string()
        } else if token
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\')
        {
            format!("\"{}\"", token.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            token.to_string()
        }
    }

    fn update_target_db(&mut self, servers: &[(String, String)]) {
        if let Some(pos) = servers.iter().position(|(a, _)| a == &self.selected_alias) {
            if let Ok(url) = ValkeyUrl::parse_valkey_url(Some(&self.selected_alias), &servers[pos].1) {
                self.target_db = url.db().unwrap_or(0) as usize;
            }
        }
    }

    fn ui_connection_settings(&mut self, ui: &mut egui::Ui, servers: &[(String, String)]) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔌 Target connection").strong());
                if !self.selected_alias.is_empty() {
                    ui.label(egui::RichText::new(format!("• DB {}", self.target_db)).weak());
                }
            });
            ui.add_space(6.0);
            if servers.is_empty() {
                ui.label("No saved connections. Go to Connections tab to add.");
            } else {
                let mut sel_idx = servers
                    .iter()
                    .position(|(a, _)| a == &self.selected_alias)
                    .unwrap_or(0);
                let old_idx = sel_idx;
                egui::ComboBox::from_id_salt("target_connection_combo")
                    .width(ui.available_width())
                    .selected_text(&self.selected_alias)
                    .show_index(ui, &mut sel_idx, servers.len(), |i| &servers[i].0);

                if sel_idx != old_idx {
                    self.selected_alias = servers[sel_idx].0.clone();
                    self.update_target_db(servers);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Target DB").strong());
                    ui.add(egui::Slider::new(&mut self.target_db, 0..=15));
                });
                ui.label(egui::RichText::new("Pick the logical database to receive the import.").small().weak());
            }
        });
    }

    fn ui_source_file(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("📁 Source file").strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.file_path)
                    .hint_text("Select RESP dump or CLI output")
                    .desired_width(ui.available_width() - 80.0));
                if ui.button("Browse…").clicked() { self.file_dialog.pick_file(); }
            });
            ui.label(egui::RichText::new("Each line should be a Valkey command (RESP inline). CSV files with keys will create empty values.").small().weak());
        });
    }

    fn start_import(&self, state: &AppState, servers: Vec<(String, String)>) {
        let (sender, i18n) = (state.get_sender(), state.i18n());
        let (is_imp, prog, logs) = (self.is_importing.clone(), self.progress.clone(), self.logs.clone());
        let (path, alias, overwrite, db) = (self.file_path.clone(), self.selected_alias.clone(), self.overwrite, self.target_db);

        std::thread::spawn(move || {
            let log = |m: String, e: bool| {
                match logs.write() {
                    Ok(mut lg) => lg.push(LogEntry { message: m, is_error: e }),
                    Err(p) => Error::from(p).log_error(),
                }
            };
            let fail = |m: String| {
                log(m.clone(), true);
                if let Ok(mut imp) = is_imp.write() {
                    *imp = false;
                }
                let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(BannerParams {
                    header: "Import failed".into(), message: m, kind: BannerKind::Error, duration_ms: 10000, request: None,
                }))));
            };

            let run = || -> Result<(), Error> {
                *is_imp.write()? = true;
                *prog.write()? = 0.0;
                logs.write()?.clear();

                log(format!("Starting import from `{}`", path), false);
                log(format!("Target: `{}` (DB {})", alias, db), false);

                let url = match servers.iter().find(|(a, _)| a == &alias) {
                    Some((_, u)) => u.clone(),
                    None => {
                        fail("Selected server not found".into());
                        return Ok(());
                    }
                };

                let client = match ValkeyClient::new(Arc::new(None), Arc::new(url), sender.clone(), i18n, false) {
                    Ok(c) => { log("Connection established".into(), false); c }
                    Err(e) => { fail(format!("Client error: {}", e)); return Ok(()); }
                };

                let content = match fs::read_to_string(&path) {
                    Ok(c) => { log(format!("File read ({} bytes)", c.len()), false); c }
                    Err(e) => { fail(format!("File error: {}", e)); return Ok(()); }
                };

                let cmds = match Self::commands_from_content(&content, &path, overwrite) {
                    Ok(c) if !c.is_empty() => { log(format!("Parsed {} commands", c.len()), false); c }
                    Ok(_) => { fail("No commands found".into()); return Ok(()); }
                    Err(e) => { fail(e); return Ok(()); }
                };

                let chunk_size = 50;
                let total = (cmds.len() + chunk_size - 1) / chunk_size;
                for (i, chunk) in cmds.chunks(chunk_size).enumerate() {
                    log(format!("Importing chunk {}/{}", i + 1, total), false);
                    match client.exec_pipelined_with_status(&chunk.to_vec()) {
                        Ok((resps, errs)) => {
                            {
                                let mut lg = logs.write()?;
                                for (cmd, resp) in chunk.iter().zip(resps.iter()) {
                                    if let Some(entry) = Self::process_import_response(cmd, resp) { lg.push(entry); }
                                }
                                for err in &errs { lg.push(LogEntry { message: format!("Error in chunk {}: {}", i + 1, err), is_error: true }); }
                            }
                            if let Some(err) = errs.first() {
                                fail(format!("Error in chunk {}: {}", i + 1, err));
                                return Ok(());
                            }
                            *prog.write()? = (i + 1) as f64 / total as f64;
                        }
                        Err(e) => {
                            fail(format!("Error in chunk {}: {}", i + 1, e));
                            return Ok(());
                        }
                    }
                }

                log("Import completed successfully".into(), false);
                let _ = sender.send(Message::Event(Arc::new(Event::ShowBanner(BannerParams {
                    header: "Import complete".into(), message: format!("Imported `{}` to `{}`", path, alias), kind: BannerKind::Success, duration_ms: 5000, request: None,
                }))));
                *is_imp.write()? = false;
                Ok(())
            };

            if let Err(e) = run() {
                e.show_error_dialog_and_reset(sender.clone(), is_imp.clone());
            }
        });
    }
}

impl Component for ImportWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        self.file_dialog.update(ctx);
        if let Some(path) = self.file_dialog.picked() { self.file_path = path.display().to_string(); }

        let servers = state.get_settings().get_servers().unwrap_or_else(|_e| {
            vec![]
        });

        if self.selected_alias.is_empty() && !servers.is_empty() {
            self.selected_alias = state.valkey_client.as_ref().and_then(|c| c.alias()).unwrap_or_else(|| servers[0].0.clone());
            self.update_target_db(&servers);
        }

        let is_importing_arc = self.is_importing.clone();
        let is_importing = *is_importing_arc.read()?;
        let progress_arc = self.progress.clone();
        let progress = *progress_arc.read()? as f32;
        let logs_arc = self.logs.clone();
        let logs = logs_arc.read()?;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("💾 Import Valkey Data");
            ui.label(egui::RichText::new("Bring dump commands into your selected Valkey connection.").weak());
            ui.add_space(12.0);

            ui.columns(2, |cols| {
                let left = &mut cols[0];
                left.set_min_width(360.0);
                left.spacing_mut().item_spacing.y = 10.0;

                self.ui_connection_settings(left, &servers);
                self.ui_source_file(left);

                egui::Frame::group(left.style()).show(left, |ui| {
                    ui.label(egui::RichText::new("⛭ Options").strong());
                    ui.checkbox(&mut self.overwrite, "Overwrite existing keys");
                    ui.label(egui::RichText::new("If disabled, existing keys remain and conflicts may fail.").small().weak());
                });

                let can_start = !self.selected_alias.trim().is_empty() && !self.file_path.trim().is_empty() && !is_importing;
                left.horizontal(|ui| {
                    let mut resp = ui.add_enabled(can_start, egui::Button::new(egui::RichText::new("▶ Start import").strong()).min_size(egui::vec2(160.0, 32.0)));
                    if !can_start && !is_importing { resp = resp.on_disabled_hover_text("Select a target connection and a source file to start."); }
                    if resp.clicked() { self.start_import(state, servers); }
                });

                let right = &mut cols[1];
                right.set_min_width(280.0);
                right.spacing_mut().item_spacing.y = 10.0;

                egui::Frame::group(right.style()).show(right, |ui| {
                    ui.set_width(ui.available_width());
                    if is_importing {
                        ui.label(egui::RichText::new("📡 Import Progress").strong());
                        ui.add(egui::ProgressBar::new(progress).show_percentage());
                        ui.add_space(12.0); ui.separator(); ui.add_space(12.0);
                    }
                    ui.label(egui::RichText::new("📝 Activity log").strong());
                    egui::ScrollArea::vertical().auto_shrink([false, true]).stick_to_bottom(true).show(ui, |ui| {
                        if logs.is_empty() { ui.label(egui::RichText::new("No log entries yet.").weak()); }
                        else {
                            for log in &*logs {
                                let color = if log.is_error { egui::Color32::RED } else { ui.visuals().text_color() };
                                ui.add(egui::Label::new(egui::RichText::new(&log.message).monospace().color(color)).selectable(false));
                            }
                        }
                    });
                });
            });
        });
        Ok(())
    }

    fn refresh(&mut self, _client: &Arc<ValkeyClient>) {}
}

#[cfg(test)]
mod tests {
    use super::ImportWindow;

    #[test]
    fn parses_csv_keys_with_overwrite() {
        let content = r#""counter","shopping_list","tags","user_hash","user:2","user:1","leaderboard""#;
        let commands = ImportWindow::commands_from_csv(content, true).unwrap();

        assert_eq!(
            commands,
            vec![
                r#"SET counter """#.to_string(),
                r#"SET shopping_list """#.to_string(),
                r#"SET tags """#.to_string(),
                r#"SET user_hash """#.to_string(),
                r#"SET user:2 """#.to_string(),
                r#"SET user:1 """#.to_string(),
                r#"SET leaderboard """#.to_string(),
            ]
        );
    }

    #[test]
    fn parses_csv_keys_without_overwrite_uses_setnx() {
        let content = r#""temp key""#;
        let commands = ImportWindow::commands_from_csv(content, false).unwrap();

        assert_eq!(commands, vec![r#"SETNX "temp key" """#.to_string()]);
    }

    #[test]
    fn parses_csv_command_row_as_command() {
        let content = r#""SET","user name","value with spaces""#;
        let commands = ImportWindow::commands_from_csv(content, true).unwrap();

        assert_eq!(commands, vec![r#"SET "user name" "value with spaces""#.to_string()]);
    }

    #[test]
    fn ignores_empty_fields_in_csv() {
        let content = r#""","SET","foo","bar""#;
        let commands = ImportWindow::commands_from_csv(content, true).unwrap();

        assert_eq!(commands, vec![r#"SET foo bar"#.to_string()]);
    }

    #[test]
    fn parses_csv_command_without_overwrite_uses_nx() {
        let content = r#""SET","foo","bar""#;
        let commands = ImportWindow::commands_from_csv(content, false).unwrap();

        assert!(commands[0].contains("NX"));
    }

    #[test]
    fn parses_plain_lines_with_overwrite_false_honors_it() {
        let content = "SET a 1\nSET b 2\nMSET c 3 d 4\nSETEX e 60 5\nSET \"\" \"\"";
        let commands = ImportWindow::commands_from_lines(content, false);

        assert!(commands[0].contains("NX"));
        assert!(commands[1].contains("NX"));
        assert!(commands[2].starts_with("MSETNX"));
        assert!(commands[3].starts_with("SET"));
        assert!(commands[3].contains("EX 60"));
        assert!(commands[3].contains("NX"));
        assert_eq!(commands[4], "SET \"\" \"\" NX");
    }

    #[test]
    fn trims_and_filters_plain_lines() {
        let content = "SET a 1\n\n INCR a ";
        let commands = ImportWindow::commands_from_lines(content, true);

        assert_eq!(commands, vec!["SET a 1".to_string(), "INCR a".to_string()]);
    }

    #[test]
    fn test_process_import_response_nx() {
        let cmd = "SET key1 val1 NX";
        let resp = ""; // nil
        let entry = ImportWindow::process_import_response(cmd, resp).unwrap();
        assert_eq!(entry.message, "Key `key1` already exists, skipping");
        assert!(!entry.is_error);

        let cmd = "SETNX key2 val2";
        let resp = "0";
        let entry = ImportWindow::process_import_response(cmd, resp).unwrap();
        assert_eq!(entry.message, "Key `key2` already exists, skipping");
        assert!(!entry.is_error);

        let cmd = "MSETNX k1 v1 k2 v2";
        let resp = "0";
        let entry = ImportWindow::process_import_response(cmd, resp).unwrap();
        assert_eq!(entry.message, "One or more keys already exists, skipping");
        assert!(!entry.is_error);
    }

    #[test]
    fn test_process_import_response_success() {
        let cmd = "SET key1 val1";
        let resp = "OK";
        let entry = ImportWindow::process_import_response(cmd, resp);
        assert!(entry.is_none());

        let cmd = "INCR count";
        let resp = "1";
        let entry = ImportWindow::process_import_response(cmd, resp);
        assert!(entry.is_none());
    }

    #[test]
    fn test_process_import_response_error() {
        let cmd = "INVALID cmd";
        let resp = "ERR unknown command";
        let entry = ImportWindow::process_import_response(cmd, resp).unwrap();
        assert!(entry.is_error);
        assert!(entry.message.contains("Response for 'INVALID cmd'"));
    }
}
