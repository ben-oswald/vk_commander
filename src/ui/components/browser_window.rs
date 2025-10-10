use crate::errors::Error;
use crate::i18n::LangKey;
use crate::state::Event::ShowError;
use crate::state::{AppState, Message};
use crate::ui::Component;
use crate::ui::widgets::{EditKey, PopupType, shimmer_inline};
use crate::utils::{KeyType, KeyTypeExtended, ValkeyClient, ValkeyUrl, format_size};
use eframe::epaint::{Rect, Stroke};
use egui::{
    Align, Button, Context, Id, Label, Layout, Modal, ScrollArea, Sense, StrokeKind, UiBuilder,
    Vec2,
};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

const SCAN_COUNT: usize = 500;
const KEY_METADATA_DEBOUNCE_MS: u64 = 300;

#[derive(Clone, Default)]
pub struct KeyMetadata {
    pub key_type: Option<KeyTypeExtended>,
    pub size: Option<u64>,
    pub ttl: Option<i64>, // -1 for no expiry, -2 for key doesn't exist, >= 0 for TTL in seconds
}

#[derive(Clone)]
enum WorkerTask {
    Keys {
        client: Arc<ValkeyClient>,
        cursor: Option<usize>,
        filter: String,
        key_type: String,
        force_run: bool,
    },
    KeyCount {
        client: Arc<ValkeyClient>,
    },
    KeyMetadata {
        client: Arc<ValkeyClient>,
        keys: Vec<String>,
        delay_ms: u64,
    },
}

#[derive(Clone)]
enum WorkerResult {
    KeysResult {
        cursor: usize,
        keys: Vec<String>,
        force_run: bool,
    },
    KeyCountResult {
        count: usize,
    },
    KeyMetadataResult {
        metadata: HashMap<String, KeyMetadata>,
    },
    Error(String),
}

struct WorkerThread {
    task_sender: mpsc::Sender<WorkerTask>,
    result_receiver: Arc<Mutex<mpsc::Receiver<WorkerResult>>>,
    _handle: thread::JoinHandle<()>,
}

impl WorkerThread {
    fn new() -> Self {
        let (task_sender, task_receiver) = mpsc::channel::<WorkerTask>();
        let (result_sender, result_receiver) = mpsc::channel::<WorkerResult>();
        let result_receiver = Arc::new(Mutex::new(result_receiver));

        let handle = thread::spawn(move || {
            while let Ok(task) = task_receiver.recv() {
                let result = Self::process_task(task);
                if result_sender.send(result).is_err() {
                    break;
                }
            }
        });

        WorkerThread {
            task_sender,
            result_receiver,
            _handle: handle,
        }
    }

    fn send_task(&self, task: WorkerTask) -> Result<(), mpsc::SendError<WorkerTask>> {
        self.task_sender.send(task)
    }

    fn try_recv_result(&self) -> Option<WorkerResult> {
        if let Ok(receiver) = self.result_receiver.try_lock() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }

    fn get_all_results(&self) -> Vec<WorkerResult> {
        let mut results = Vec::new();
        while let Some(result) = self.try_recv_result() {
            results.push(result);
        }
        results
    }

    fn process_task(task: WorkerTask) -> WorkerResult {
        match task {
            WorkerTask::Keys {
                client,
                cursor,
                filter,
                key_type,
                force_run,
            } => {
                let command = format!(
                    "SCAN {}{} COUNT {SCAN_COUNT} {}",
                    cursor.unwrap_or(0),
                    if filter.is_empty() {
                        "".to_string()
                    } else {
                        format!(" MATCH {filter}")
                    },
                    key_type
                );

                match client.exec(command.trim()) {
                    Ok(mut res) => {
                        let new_cursor = res.remove(0).parse::<usize>().unwrap_or(0);
                        WorkerResult::KeysResult {
                            cursor: new_cursor,
                            keys: res,
                            force_run,
                        }
                    }
                    Err(e) => WorkerResult::Error(format!("Failed to get keys: {e:?}")),
                }
            }
            WorkerTask::KeyCount { client } => match client.exec("DBSIZE") {
                Ok(res) => {
                    let count = res
                        .first()
                        .unwrap_or(&String::from("0"))
                        .parse::<usize>()
                        .unwrap_or(0);
                    WorkerResult::KeyCountResult { count }
                }
                Err(e) => WorkerResult::Error(format!("Failed to get key count: {e:?}")),
            },
            WorkerTask::KeyMetadata {
                client,
                keys,
                delay_ms,
            } => {
                if keys.is_empty() {
                    return WorkerResult::KeyMetadataResult {
                        metadata: HashMap::new(),
                    };
                }

                if delay_ms > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }

                let mut metadata = HashMap::new();

                let type_commands: Vec<String> = keys
                    .iter()
                    .map(|key| {
                        if key.contains(' ')
                            || key.contains('"')
                            || key.contains('\t')
                            || key.contains('\n')
                            || key.contains('\r')
                        {
                            let escaped_key = key.replace('"', "\\\"");
                            format!("TYPE \"{escaped_key}\"")
                        } else {
                            format!("TYPE {key}")
                        }
                    })
                    .collect();

                let ttl_commands: Vec<String> = keys
                    .iter()
                    .map(|key| {
                        if key.contains(' ')
                            || key.contains('"')
                            || key.contains('\t')
                            || key.contains('\n')
                            || key.contains('\r')
                        {
                            let escaped_key = key.replace('"', "\\\"");
                            format!("TTL \"{escaped_key}\"")
                        } else {
                            format!("TTL {key}")
                        }
                    })
                    .collect();

                let types_result = client.exec_pipelined(&type_commands);
                let ttl_result = client.exec_pipelined(&ttl_commands);

                match (types_result, ttl_result) {
                    (Ok(types), Ok(ttls)) => {
                        let size_commands: Vec<String> = keys
                            .iter()
                            .enumerate()
                            .filter_map(|(i, key)| {
                                let raw_key_type = types.get(i)?;
                                if raw_key_type != "none" && raw_key_type != "unknown" {
                                    let key_type_extended = KeyTypeExtended::from(raw_key_type);
                                    let key_type: KeyType = key_type_extended.into();

                                    let quoted_key = if key.contains(' ')
                                        || key.contains('"')
                                        || key.contains('\t')
                                        || key.contains('\n')
                                        || key.contains('\r')
                                    {
                                        let escaped_key = key.replace('"', "\\\"");
                                        format!("\"{escaped_key}\"")
                                    } else {
                                        key.to_string()
                                    };

                                    Some(match key_type {
                                        KeyType::Hash
                                        | KeyType::List
                                        | KeyType::Set
                                        | KeyType::SortedSet
                                        | KeyType::String => {
                                            format!("MEMORY USAGE {quoted_key}")
                                        }
                                        KeyType::Bloom => format!("BF.INFO {quoted_key} SIZE"),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let sizes_result = if !size_commands.is_empty() {
                            client.exec_pipelined(&size_commands).ok()
                        } else {
                            Some(Vec::new())
                        };

                        let mut size_idx = 0;
                        for (i, key) in keys.iter().enumerate() {
                            let raw_key_type = &types
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string());
                            let ttl = ttls
                                .get(i)
                                .and_then(|t| t.parse::<i64>().ok())
                                .unwrap_or(-2);

                            let key_type_extended = KeyTypeExtended::from(&raw_key_type);

                            let size = if raw_key_type != "none" && raw_key_type != "unknown" {
                                if let Some(ref sizes) = sizes_result {
                                    let result =
                                        sizes.get(size_idx).and_then(|s| s.parse::<u64>().ok());
                                    size_idx += 1;
                                    result
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            metadata.insert(
                                key.clone(),
                                KeyMetadata {
                                    key_type: Some(key_type_extended),
                                    size,
                                    ttl: Some(ttl),
                                },
                            );
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        return WorkerResult::Error(format!("Failed to get key metadata: {e:?}"));
                    }
                }

                WorkerResult::KeyMetadataResult { metadata }
            }
        }
    }
}

#[derive(Default)]
pub struct BrowserWindow {
    selected_key_type_idx: usize,
    key_filter: (String, String),
    cursor: Option<usize>,
    keys: Vec<String>,
    key_metadata: HashMap<String, KeyMetadata>,
    pending_metadata_keys: HashSet<String>,
    last_visible_row: Option<usize>,
    initial: bool,
    key_count: Option<usize>,
    row_range: (Range<usize>, Range<usize>),
    worker: Option<WorkerThread>,
    pending_key_request: bool,
    pending_count_request: bool,
    last_metadata_request: Option<Instant>,
    pending_metadata_range: Option<Range<usize>>,
    key_to_delete: Option<String>,
    key_to_rename: Option<(String, String)>,
    ttl_to_set: Option<(String, String)>,
    pending_key_edits: Vec<(String, KeyMetadata)>,
    loading_key_edit: bool,
}

impl Component for BrowserWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        if self.worker.is_none() {
            self.worker = Some(WorkerThread::new());
        }

        if let Some(worker) = &self.worker {
            let results = worker.get_all_results();
            for result in results {
                self.handle_worker_result(result);
            }
        }

        if let Some(valkey_client) = &state.valkey_client {
            let server_info = ValkeyUrl::from(&*valkey_client.clone());
            let key_type_label = state.i18n().get(LangKey::KeyType);

            if !self.pending_key_edits.is_empty() {
                let pending_edits = std::mem::take(&mut self.pending_key_edits);
                let mut successfully_processed = Vec::new();
                let mut still_pending = Vec::new();

                for (key_name, _old_metadata) in pending_edits {
                    let fresh_metadata = self
                        .key_metadata
                        .get(&key_name)
                        .cloned()
                        .unwrap_or_default();

                    if fresh_metadata.key_type.is_some() {
                        self.edit_key(state, &fresh_metadata, &key_name);
                        successfully_processed.push(key_name);
                    } else {
                        still_pending.push((key_name, fresh_metadata));
                    }
                }

                self.pending_key_edits = still_pending;
                self.loading_key_edit = !self.pending_key_edits.is_empty();
            }

            if self.key_count.is_none() && !self.pending_count_request {
                self.request_key_count(valkey_client);
            }

            let key_count = self.key_count.unwrap_or(0);
            let mut needs_key_refresh = false;

            if self.key_filter.0 != self.key_filter.1 {
                self.request_keys(valkey_client, true);
                self.key_filter.1 = self.key_filter.0.clone();
            }

            let keys_len = self.keys.len();

            if self.row_range.0 != self.row_range.1 {
                let start = self.row_range.0.start.min(keys_len);
                let end = self.row_range.0.end.min(keys_len);

                if start < end {
                    let keys_needing_metadata: Vec<String> = self.keys[start..end]
                        .iter()
                        .filter(|key| {
                            !self.key_metadata.contains_key(*key)
                                && !self.pending_metadata_keys.contains(*key)
                        })
                        .cloned()
                        .collect();

                    if !keys_needing_metadata.is_empty() {
                        let now = Instant::now();
                        let should_request = if let Some(last_request) = self.last_metadata_request
                        {
                            now.duration_since(last_request)
                                >= Duration::from_millis(KEY_METADATA_DEBOUNCE_MS)
                        } else {
                            true
                        };

                        if should_request {
                            self.request_key_metadata(valkey_client, &keys_needing_metadata);
                            self.last_metadata_request = Some(now);
                            self.pending_metadata_range = None;
                        } else {
                            self.pending_metadata_range = Some(start..end);
                        }
                    }
                }
                self.row_range.1 = self.row_range.0.clone();
            }

            if let Some(pending_range) = &self.pending_metadata_range
                && let Some(last_request) = self.last_metadata_request
            {
                let now = Instant::now();
                if now.duration_since(last_request)
                    >= Duration::from_millis(KEY_METADATA_DEBOUNCE_MS)
                {
                    let range = pending_range.clone();
                    if range.start < keys_len && range.end <= keys_len {
                        let keys_needing_metadata: Vec<String> = self.keys[range]
                            .iter()
                            .filter(|key| {
                                !self.key_metadata.contains_key(*key)
                                    && !self.pending_metadata_keys.contains(*key)
                            })
                            .cloned()
                            .collect();

                        if !keys_needing_metadata.is_empty() {
                            self.request_key_metadata(valkey_client, &keys_needing_metadata);
                            self.last_metadata_request = Some(now);
                        }
                        self.pending_metadata_range = None;
                    }
                }
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                egui::Sides::new().show(
                    ui,
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(valkey_client.alias().unwrap_or(server_info.address()));
                            ui.separator();
                            ui.heading(format!("{}: {key_count}", state.i18n().get(LangKey::Keys)));
                        })
                    },
                    |ui| {
                        if ui.button("↻").clicked() {
                            self.refresh(valkey_client);
                        }
                    },
                );
                ui.separator();

                ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                    self.key_type_selector(ui, valkey_client, key_type_label)
                        .unwrap_or_else(|e| {
                            e.show_error_dialog(state.get_sender());
                        });
                    ui.add_sized(
                        [ui.available_width() - 100.0, 0.0],
                        egui::TextEdit::singleline(&mut self.key_filter.0)
                            .hint_text(state.i18n().get(LangKey::FilterByKeyNameOrPattern)),
                    );
                    if ui
                        .add_sized([80.0, 0.0], Button::new(state.i18n().get(LangKey::Add)))
                        .clicked()
                    {
                        let sender = state.get_sender();
                        sender
                            .send(Message::OpenPopup(PopupType::AddKey(Box::default())))
                            .unwrap_or_else(|e| Error::from(e).show_error_dialog(sender.clone()));
                    };
                });

                ui.separator();

                if self.loading_key_edit {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label(format!("{}...", state.i18n().get(LangKey::LoadingKeyData)));
                    });
                    ui.separator();
                }

                ui.horizontal(|ui| {
                    ui.add_sized([40.0, 20.0], Label::new(state.i18n().get(LangKey::Index)));
                    ui.add_sized([60.0, 20.0], Label::new(state.i18n().get(LangKey::Type)));
                    ui.add_sized([60.0, 20.0], Label::new(state.i18n().get(LangKey::Size)));
                    ui.add_sized([80.0, 20.0], Label::new(state.i18n().get(LangKey::Ttl)));
                    ui.add_sized(
                        [ui.available_width() - 10.0, 20.0],
                        Label::new(state.i18n().get(LangKey::Key)),
                    );
                });
                ui.separator();

                let available_height = ui.available_height();
                let row_height = 32.0;

                let mut collected_key_edits = Vec::new();
                ScrollArea::vertical()
                    .id_salt("browser_keys_list_scroll")
                    .max_height(available_height)
                    .show_rows(ui, row_height - 2.0, keys_len, |ui, row_range| {
                        self.row_range.0 = row_range.clone();
                        let key_edit_requests =
                            self.list_items(state, ui, row_range, row_height)?;
                        collected_key_edits.extend(key_edit_requests);
                        Ok::<(), Error>(())
                    });

                for (key_name, metadata) in collected_key_edits {
                    self.edit_key(state, &metadata, &key_name);
                }

                if !self.initial {
                    needs_key_refresh = true;
                    self.initial = true;
                }

                if let Some(bottom) = self.last_visible_row
                    && bottom < 1000
                    && bottom + 1 >= keys_len
                {
                    needs_key_refresh = true;
                }

                if self.key_to_delete.is_some()
                    || self.key_to_rename.is_some()
                    || self.ttl_to_set.is_some()
                {
                    let modal = Modal::new(Id::new("edit_key")).show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.add(
                                        Label::new(
                                            egui::RichText::new(
                                                if let Some(key_to_rename) = &self.key_to_rename {
                                                    format!(
                                                        "{} {}?",
                                                        state.i18n().get(LangKey::Rename),
                                                        key_to_rename.0
                                                    )
                                                } else if let Some(ttl_to_set) = &self.ttl_to_set {
                                                    format!(
                                                        "{} {}?",
                                                        state.i18n().get(LangKey::SetTtlFor),
                                                        ttl_to_set.0
                                                    )
                                                } else if let Some(key_to_delete) =
                                                    &self.key_to_delete
                                                {
                                                    format!(
                                                        "{} {key_to_delete}?",
                                                        state.i18n().get(LangKey::Delete)
                                                    )
                                                } else {
                                                    state.i18n().get(LangKey::DeleteKey)
                                                },
                                            )
                                            .heading(),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.centered_and_justified(|ui| {
                                    if let Some(key_to_rename) = &mut self.key_to_rename {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut key_to_rename.1)
                                                .desired_width(ui.available_width())
                                                .hint_text(state.i18n().get(LangKey::NewKeyName)),
                                        )
                                    } else if let Some(ttl_to_set) = &mut self.ttl_to_set {
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut ttl_to_set.1)
                                                .desired_width(ui.available_width())
                                                .hint_text("-1"),
                                        );

                                        if response.changed() {
                                            ttl_to_set.1 = ttl_to_set
                                                .1
                                                .chars()
                                                .filter(|c| c.is_ascii_digit())
                                                .collect();
                                        }

                                        response
                                    } else {
                                        ui.label(format!(
                                            "{}?",
                                            state.i18n().get(LangKey::AreYouSure)
                                        ))
                                    }
                                });
                            });
                        });
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    if self.key_to_rename.is_some() || self.ttl_to_set.is_some() {
                                        state.i18n().get(LangKey::Cancel)
                                    } else {
                                        state.i18n().get(LangKey::No)
                                    },
                                )
                                .clicked()
                            {
                                self.key_to_delete = None;
                                self.key_to_rename = None;
                                self.ttl_to_set = None;
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(if self.key_to_rename.is_some() {
                                        state.i18n().get(LangKey::Rename)
                                    } else if self.ttl_to_set.is_some() {
                                        state.i18n().get(LangKey::SetTtl)
                                    } else {
                                        state.i18n().get(LangKey::Yes)
                                    })
                                    .clicked()
                                {
                                    let sender = state.get_sender();
                                    if let Some(client) = state.valkey_client.clone() {
                                        let command = if let Some(key_to_rename) =
                                            &self.key_to_rename
                                        {
                                            format!(
                                                "RENAME {} {}",
                                                key_to_rename.0, key_to_rename.1
                                            )
                                        } else if let Some(ttl_to_set) = &self.ttl_to_set {
                                            let ttl_value =
                                                ttl_to_set.1.parse::<i64>().unwrap_or(-1);
                                            if ttl_value == -1 {
                                                format!("PERSIST {}", ttl_to_set.0)
                                            } else {
                                                format!("EXPIRE {} {ttl_value}", ttl_to_set.0)
                                            }
                                        } else if let Some(key_to_delete) = &self.key_to_delete {
                                            format!("DEL {key_to_delete}")
                                        } else {
                                            "PING".into()
                                        };
                                        let i18n = state.i18n();
                                        thread::spawn(move || {
                                            client.exec(&command).unwrap_or_else(|e| {
                                                e.show_error_dialog(sender.clone());
                                                vec![]
                                            });
                                            sender.send(Message::Refresh).unwrap_or_else(|e| {
                                                eprintln!(
                                                    "{}: {e}",
                                                    i18n.get(LangKey::ErrorSendingRefreshWinMsg)
                                                )
                                            });
                                        });
                                    } else {
                                        sender
                                            .send(Message::Event(Arc::from(ShowError(
                                                Error::from(
                                                    state.i18n().get(LangKey::CantAccessValkeyDb),
                                                ),
                                            ))))
                                            .unwrap_or_else(|e| {
                                                eprintln!(
                                                    "{}: {e}",
                                                    state.i18n().get(LangKey::ErrorSendMsg)
                                                )
                                            });
                                    }
                                    self.key_to_delete = None;
                                    self.key_to_rename = None;
                                    self.ttl_to_set = None;
                                }
                            });
                        });
                    });
                    if modal.should_close() {
                        self.key_to_delete = None;
                        self.key_to_rename = None;
                        self.ttl_to_set = None;
                    }
                }
            });

            if needs_key_refresh {
                self.request_keys(valkey_client, false);
            }

            ctx.request_repaint();
        }
        Ok(())
    }
    fn refresh(&mut self, valkey_client: &Arc<ValkeyClient>) {
        self.keys.clear();
        self.key_metadata.clear();
        self.pending_metadata_keys.clear();
        self.cursor = None;

        self.request_key_count(valkey_client);
        self.request_keys(valkey_client, true);

        let keys_len = self.keys.len();
        let start = self.row_range.0.start.min(keys_len);
        let end = self.row_range.0.end.min(keys_len);

        if start < end && !self.keys.is_empty() {
            let visible_keys: Vec<String> = self.keys[start..end].to_vec();
            if !visible_keys.is_empty() {
                self.request_key_metadata(valkey_client, &visible_keys);
            }
        }
    }
}

impl BrowserWindow {
    fn handle_worker_result(&mut self, result: WorkerResult) {
        match result {
            WorkerResult::KeysResult {
                cursor,
                mut keys,
                force_run,
            } => {
                self.cursor = Some(cursor);
                self.pending_key_request = false;

                if force_run {
                    self.keys.clear();
                }
                self.keys.append(&mut keys);
            }
            WorkerResult::KeyCountResult { count } => {
                self.key_count = Some(count);
                self.pending_count_request = false;
            }
            WorkerResult::KeyMetadataResult { metadata } => {
                for key in metadata.keys() {
                    self.pending_metadata_keys.remove(key);
                }
                self.key_metadata.extend(metadata);
            }
            WorkerResult::Error(error) => {
                eprintln!("Worker error: {error}");
                self.pending_key_request = false;
                self.pending_count_request = false;
                self.pending_metadata_keys.clear();
            }
        }
    }

    fn request_keys(&mut self, valkey_client: &Arc<ValkeyClient>, force_run: bool) {
        if self.pending_key_request && !force_run {
            return;
        }

        let cursor_value = if force_run {
            self.cursor = None;
            if force_run {
                self.keys.clear();
                self.pending_metadata_keys.clear();
            }
            None
        } else {
            if let Some(cursor_val) = self.cursor
                && cursor_val == 0
            {
                return;
            }
            self.cursor
        };

        if let Some(worker) = &self.worker {
            let task = WorkerTask::Keys {
                client: Arc::clone(valkey_client),
                cursor: cursor_value,
                filter: self.key_filter.0.clone(),
                key_type: KeyTypeExtended::vector()[self.selected_key_type_idx]
                    .to_resp_str()
                    .to_string(),
                force_run,
            };

            if worker.send_task(task).is_ok() {
                self.pending_key_request = true;
            }
        }
    }

    fn request_key_count(&mut self, valkey_client: &Arc<ValkeyClient>) {
        if self.pending_count_request {
            return;
        }

        if let Some(worker) = &self.worker {
            let task = WorkerTask::KeyCount {
                client: Arc::clone(valkey_client),
            };

            if worker.send_task(task).is_ok() {
                self.pending_count_request = true;
            }
        }
    }

    fn request_key_metadata(&mut self, valkey_client: &Arc<ValkeyClient>, keys: &[String]) {
        if keys.is_empty() {
            return;
        }

        for key in keys {
            self.pending_metadata_keys.insert(key.clone());
        }

        if let Some(worker) = &self.worker {
            let task = WorkerTask::KeyMetadata {
                client: Arc::clone(valkey_client),
                keys: keys.to_vec(),
                delay_ms: KEY_METADATA_DEBOUNCE_MS,
            };

            let _ = worker.send_task(task);
        }
    }

    fn key_type_selector(
        &mut self,
        ui: &mut egui::Ui,
        valkey_client: &Arc<ValkeyClient>,
        key_type_label: String,
    ) -> Result<(), Error> {
        ui.horizontal(|ui| {
            ui.label(key_type_label);
            egui::ComboBox::new(ui.id().with("key_type"), "")
                .width(100.0)
                .selected_text(KeyTypeExtended::vector()[self.selected_key_type_idx].to_string())
                .show_ui(ui, |ui| {
                    for (i, key_type) in KeyTypeExtended::vector().iter().enumerate() {
                        if ui
                            .selectable_value(
                                &mut self.selected_key_type_idx,
                                i,
                                key_type.to_string(),
                            )
                            .changed()
                        {
                            self.request_keys(valkey_client, true);
                        }
                    }
                });
        });
        Ok(())
    }

    fn format_ttl(&self, ttl: Option<i64>) -> String {
        match ttl {
            Some(-1) => "∞".to_string(),
            Some(-2) => "N/A".to_string(),
            Some(t) if t >= 0 => {
                if t < 60 {
                    format!("{t}s")
                } else if t < 3600 {
                    format!("{}m", t / 60)
                } else if t < 86400 {
                    format!("{}h", t / 3600)
                } else {
                    format!("{}d", t / 86400)
                }
            }
            _ => "...".to_string(),
        }
    }

    fn list_items(
        &mut self,
        state: &AppState,
        ui: &mut egui::Ui,
        row_range: Range<usize>,
        row_height: f32,
    ) -> Result<Vec<(String, KeyMetadata)>, Error> {
        let keys = &self.keys;
        let max_w = ui.available_width();
        let start = row_range.start as f32;
        let end = row_range.end;
        let mut key_edit_requests = Vec::new();

        for idx in row_range {
            if idx >= keys.len() {
                break;
            }
            let x = ui.min_rect().min.x;
            let y = ui.min_rect().min.y + (idx as f32 - start) * row_height;
            let row_rect = Rect::from_min_size([x, y].into(), Vec2::new(max_w, row_height));
            let key_name = self.keys[idx].clone();

            let metadata = self
                .key_metadata
                .get(&keys[idx])
                .cloned()
                .unwrap_or_default();

            let (rect, resp) =
                ui.allocate_exact_size(row_rect.size(), Sense::click().union(Sense::hover()));

            let visuals = if resp.clicked() {
                ui.style().visuals.extreme_bg_color
            } else if resp.hovered() {
                ui.style().visuals.widgets.hovered.bg_fill
            } else if idx % 2 == 1 {
                ui.style().visuals.faint_bg_color
            } else {
                ui.style().visuals.widgets.noninteractive.bg_fill
            };

            if resp.clicked() {
                key_edit_requests.push((key_name.clone(), metadata.clone()));
            }
            resp.context_menu(|ui| {
                if ui
                    .add(Button::new(state.i18n().get(LangKey::Copy)))
                    .clicked()
                {
                    ui.ctx().copy_text(key_name.clone());
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(Button::new(state.i18n().get(LangKey::Edit)))
                    .clicked()
                {
                    key_edit_requests.push((key_name.clone(), metadata.clone()));
                    ui.close();
                }
                if ui
                    .add(Button::new(state.i18n().get(LangKey::Rename)))
                    .clicked()
                {
                    self.key_to_rename = Some((key_name.clone(), key_name.clone()));
                    ui.close();
                }
                if ui
                    .add(Button::new(state.i18n().get(LangKey::SetTtl)))
                    .clicked()
                {
                    let ttl = metadata.ttl.unwrap_or(-1);
                    self.ttl_to_set = Some((
                        key_name.clone(),
                        if ttl < 0 {
                            "".to_string()
                        } else {
                            ttl.to_string()
                        },
                    ));
                    ui.close();
                }
                if ui
                    .add(Button::new(state.i18n().get(LangKey::Delete)))
                    .clicked()
                {
                    self.key_to_delete = Some(key_name);
                    ui.close();
                }
            });

            ui.painter().rect(
                rect,
                ui.style().visuals.widgets.noninteractive.corner_radius,
                visuals,
                Stroke::NONE,
                StrokeKind::Outside,
            );

            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.set_min_size([0.0, row_height].into());

                    ui.add_sized(
                        [40.0, row_height],
                        Label::new(format!("{idx}")).selectable(false),
                    );

                    //TODO! refac
                    if let Some(key_type) = metadata.key_type.as_ref() {
                        ui.add_sized(
                            [60.0, row_height],
                            Label::new(key_type.to_string()).selectable(false),
                        );
                    } else {
                        shimmer_inline(ui, 60.0);
                    }

                    if let Some(size) = metadata.size {
                        let size_text = format_size(size);
                        ui.add_sized([60.0, row_height], Label::new(size_text).selectable(false));
                    } else {
                        shimmer_inline(ui, 60.0);
                    }

                    if let Some(ttl) = metadata.ttl {
                        let ttl_text = self.format_ttl(Some(ttl));
                        ui.add_sized([80.0, row_height], Label::new(ttl_text).selectable(false));
                    } else {
                        shimmer_inline(ui, 80.0);
                    }

                    ui.allocate_ui_with_layout(
                        [ui.available_width() - 10.0, row_height].into(),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(Label::new(&keys[idx]).selectable(false).truncate());
                        },
                    );
                });
            });
        }

        let last_visible = end.saturating_sub(1);
        self.last_visible_row = Some(last_visible);
        Ok(key_edit_requests)
    }

    fn edit_key(&mut self, state: &AppState, metadata: &KeyMetadata, key_name: &str) {
        let client = state.valkey_client.clone();
        let sender = state.get_sender();
        let key_name = key_name.to_owned();

        if let Some(client) = client {
            if let Some(key_type_extended) = metadata.key_type {
                let i18n = state.i18n();
                thread::spawn(move || {
                    let quoted_key = if key_name.contains(' ')
                        || key_name.contains('"')
                        || key_name.contains('\t')
                        || key_name.contains('\n')
                        || key_name.contains('\r')
                    {
                        let escaped_key = key_name.replace('"', "\\\"");
                        format!("\"{escaped_key}\"")
                    } else {
                        key_name.clone()
                    };

                    let (command, key_type) = match key_type_extended {
                        KeyTypeExtended::KeyType(kt) => match kt {
                            KeyType::Hash => (format!("HGETALL {quoted_key}"), kt),
                            KeyType::List => (format!("LRANGE {quoted_key} 0 499"), kt),
                            KeyType::Set => (format!("SMEMBERS {quoted_key}",), kt),
                            KeyType::SortedSet => {
                                (format!("ZRANGE {quoted_key} 0 499 WITHSCORES"), kt)
                            }
                            KeyType::String => (format!("GET {quoted_key}"), kt),
                            KeyType::Bloom => (format!("BF.INFO {quoted_key}"), kt),
                        },
                        _ => {
                            sender
                                .send(Message::Event(Arc::new(ShowError(Error::from(
                                    i18n.get(LangKey::UnknownKeyType),
                                )))))
                                .unwrap_or_else(|e| {
                                    eprintln!("Error sending message: {e}");
                                });
                            return;
                        }
                    };

                    match client.exec(&command) {
                        Ok(res) => {
                            sender
                                .send(Message::OpenPopup(PopupType::EditKey(Box::new(
                                    EditKey::new(key_name, key_type, res, i18n),
                                ))))
                                .unwrap_or_else(|e| {
                                    Error::from(e).show_error_dialog(sender.clone())
                                });
                        }
                        Err(e) => {
                            e.show_error_dialog(sender.clone());
                        }
                    }
                });
            } else {
                self.pending_key_edits
                    .push((key_name.clone(), metadata.clone()));
                self.loading_key_edit = true;
            }
        } else {
            self.pending_key_edits
                .push((key_name.clone(), metadata.clone()));
            self.loading_key_edit = true;
        }
    }
}
