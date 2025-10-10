use crate::errors::Error;
use crate::i18n::I18N;
use crate::state::{AppState, Message};
use crate::ui::Component;
use crate::utils::{ValkeyClient, format_size, type_color};
use egui::{Context, ScrollArea};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

const SCAN_COUNT: usize = 100;
const MAX_KEYS_TO_ANALYZE: usize = 10000;

fn write_lock_arc<T, F>(lock: &Arc<RwLock<T>>, f: F, sender: &Arc<Sender<Message>>)
where
    F: FnOnce(&mut T),
{
    match lock.write() {
        Ok(mut guard) => f(&mut *guard),
        Err(e) => {
            Error::from(e).show_error_dialog(sender.clone());
        }
    }
}

#[derive(Clone, Debug)]
struct KeyInfo {
    name: String,
    key_type: String,
    size: u64,
    ttl: i64, // -1 for no expiry, -2 for doesn't exist, >= 0 for seconds
}

#[derive(Clone, Debug, Default)]
struct TypeStats {
    count: usize,
    total_memory: u64,
}

#[derive(Clone, Debug, Default)]
struct KeyAnalysis {
    type_stats: HashMap<String, TypeStats>,
    top_keys: Vec<KeyInfo>,
    ttl_buckets: TtlBuckets,
    last_analysis: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
struct TtlBuckets {
    hour_1: u64,
    hour_4: u64,
    hour_24: u64,
    hour_48: u64,
    hour_72: u64,
    week_1: u64,
    month_1: u64,
    month_plus: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    BySize,
    ByLength,
}

pub struct InsightsWindow {
    stats: Arc<RwLock<HashMap<String, String>>>,
    last_update: Arc<RwLock<Instant>>,
    is_fetching: Arc<RwLock<bool>>,

    key_analysis: Arc<RwLock<KeyAnalysis>>,
    is_analyzing: Arc<RwLock<bool>>,

    show_graphs: bool,
    sort_mode: SortMode,
    sender: Arc<Sender<Message>>,
    i18n: Arc<I18N>,
}

impl InsightsWindow {
    pub fn new(sender: Arc<Sender<Message>>, i18n: Arc<I18N>) -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            is_fetching: Arc::new(RwLock::new(false)),
            key_analysis: Arc::new(RwLock::new(KeyAnalysis::default())),
            is_analyzing: Arc::new(RwLock::new(false)),
            show_graphs: true,
            sort_mode: SortMode::BySize,
            sender,
            i18n,
        }
    }

    fn read_lock<T, F, R>(&self, lock: &Arc<RwLock<T>>, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        match lock.read() {
            Ok(guard) => Some(f(&*guard)),
            Err(e) => {
                Error::from(e).show_error_dialog(self.sender.clone());
                None
            }
        }
    }

    fn write_lock<T, F, R>(&self, lock: &Arc<RwLock<T>>, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        match lock.write() {
            Ok(mut guard) => Some(f(&mut *guard)),
            Err(e) => {
                Error::from(e).show_error_dialog(self.sender.clone());
                None
            }
        }
    }

    fn fetch_stats(&self, valkey_client: Arc<ValkeyClient>, ctx: &Context) {
        let stats = Arc::clone(&self.stats);
        let is_fetching = Arc::clone(&self.is_fetching);
        let last_update = Arc::clone(&self.last_update);
        let ctx = ctx.clone();
        let sender = Arc::clone(&self.sender);

        if let Some(is_currently_fetching) = self.read_lock(&is_fetching, |f| *f) {
            if is_currently_fetching {
                return;
            }
        } else {
            return;
        }

        if self.write_lock(&is_fetching, |f| *f = true).is_none() {
            return;
        }

        thread::spawn(move || {
            let mut new_stats = HashMap::new();

            if let Ok(info_result) = valkey_client.exec("INFO")
                && let Some(info_str) = info_result.first()
            {
                for line in info_str.lines() {
                    if line.contains(':') && !line.starts_with('#') {
                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            new_stats.insert(parts[0].to_string(), parts[1].trim().to_string());
                        }
                    }
                }
            }

            if let Ok(dbsize_result) = valkey_client.exec("DBSIZE")
                && let Some(dbsize_str) = dbsize_result.first()
            {
                new_stats.insert("dbsize".to_string(), dbsize_str.clone());
            }

            if let Ok(client_list_result) = valkey_client.exec("CLIENT LIST")
                && let Some(list_str) = client_list_result.first()
            {
                let count = list_str.lines().filter(|l| !l.is_empty()).count();
                new_stats.insert("connected_clients_count".to_string(), count.to_string());
            }

            write_lock_arc(&stats, |s| *s = new_stats, &sender);
            write_lock_arc(&last_update, |l| *l = Instant::now(), &sender);
            write_lock_arc(&is_fetching, |f| *f = false, &sender);

            ctx.request_repaint();
        });
    }

    fn analyze_keys(&self, valkey_client: Arc<ValkeyClient>) {
        let is_analyzing = Arc::clone(&self.is_analyzing);
        let key_analysis = Arc::clone(&self.key_analysis);
        let sender = Arc::clone(&self.sender);

        if let Some(is_currently_analyzing) = self.read_lock(&is_analyzing, |a| *a) {
            if is_currently_analyzing {
                return;
            }
        } else {
            return;
        }

        if self.write_lock(&is_analyzing, |a| *a = true).is_none() {
            return;
        }

        thread::spawn(move || {
            let mut type_stats: HashMap<String, TypeStats> = HashMap::new();
            let mut all_keys: Vec<KeyInfo> = Vec::new();
            let mut ttl_buckets = TtlBuckets::default();
            let mut cursor = 0;
            let mut total_scanned = 0;

            loop {
                if total_scanned >= MAX_KEYS_TO_ANALYZE {
                    break;
                }

                let scan_cmd = format!("SCAN {} COUNT {}", cursor, SCAN_COUNT);
                let scan_result = match valkey_client.exec(&scan_cmd) {
                    Ok(result) => result,
                    Err(_) => break,
                };

                if scan_result.len() < 2 {
                    break;
                }

                cursor = scan_result[0].parse::<usize>().unwrap_or(0);
                let keys: Vec<String> = scan_result[1..]
                    .iter()
                    .filter(|k| !k.is_empty())
                    .map(|k| k.to_string())
                    .collect();

                if keys.is_empty() {
                    if cursor == 0 {
                        break;
                    }
                    continue;
                }

                let mut type_commands = Vec::new();
                let mut ttl_commands = Vec::new();
                let mut memory_commands = Vec::new();

                for key in &keys {
                    let quoted_key = if key.contains(' ') || key.contains('"') {
                        let escaped = key.replace('"', "\\\"");
                        format!("\"{}\"", escaped)
                    } else {
                        key.clone()
                    };

                    type_commands.push(format!("TYPE {}", quoted_key));
                    ttl_commands.push(format!("TTL {}", quoted_key));
                    memory_commands.push(format!("MEMORY USAGE {}", quoted_key));
                }

                let types = valkey_client
                    .exec_pipelined(&type_commands)
                    .unwrap_or_default();
                let ttls = valkey_client
                    .exec_pipelined(&ttl_commands)
                    .unwrap_or_default();
                let sizes = valkey_client
                    .exec_pipelined(&memory_commands)
                    .unwrap_or_default();

                for (i, key) in keys.iter().enumerate() {
                    let key_type = types
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| "string".to_string());
                    let ttl = ttls
                        .get(i)
                        .and_then(|t| t.parse::<i64>().ok())
                        .unwrap_or(-1);
                    let size = sizes
                        .get(i)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);

                    let stats = type_stats.entry(key_type.clone()).or_default();
                    stats.count += 1;
                    stats.total_memory += size;

                    all_keys.push(KeyInfo {
                        name: key.clone(),
                        key_type: key_type.clone(),
                        size,
                        ttl,
                    });

                    if ttl > 0 {
                        let ttl_seconds = ttl as u64;
                        if ttl_seconds <= 3600 {
                            ttl_buckets.hour_1 += size;
                        } else if ttl_seconds <= 14400 {
                            ttl_buckets.hour_4 += size;
                        } else if ttl_seconds <= 86400 {
                            ttl_buckets.hour_24 += size;
                        } else if ttl_seconds <= 172800 {
                            ttl_buckets.hour_48 += size;
                        } else if ttl_seconds <= 259200 {
                            ttl_buckets.hour_72 += size;
                        } else if ttl_seconds <= 604800 {
                            ttl_buckets.week_1 += size;
                        } else if ttl_seconds <= 2592000 {
                            ttl_buckets.month_1 += size;
                        } else {
                            ttl_buckets.month_plus += size;
                        }
                    }
                }

                total_scanned += keys.len();

                if cursor == 0 {
                    break;
                }
            }

            all_keys.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
            let top_keys = all_keys.into_iter().take(20).collect();

            write_lock_arc(
                &key_analysis,
                |analysis| {
                    analysis.type_stats = type_stats;
                    analysis.top_keys = top_keys;
                    analysis.ttl_buckets = ttl_buckets;
                    analysis.last_analysis = Some(Instant::now());
                },
                &sender,
            );

            write_lock_arc(&is_analyzing, |a| *a = false, &sender);
        });
    }

    fn render_pie_chart_memory(&self, ui: &mut egui::Ui, type_stats: &HashMap<String, TypeStats>) {
        ui.heading("Memory Usage by Key Type");
        ui.add_space(5.0);

        if type_stats.is_empty() {
            ui.label("No data available");
            return;
        }

        let total_memory: u64 = type_stats.values().map(|s| s.total_memory).sum();
        if total_memory == 0 {
            ui.label("No memory usage data available");
            return;
        }

        let mut sorted_types: Vec<_> = type_stats.iter().collect();
        sorted_types.sort_by(|a, b| b.1.total_memory.cmp(&a.1.total_memory));

        ui.vertical(|ui| {
            for (key_type, stats) in sorted_types {
                let percentage = (stats.total_memory as f64 / total_memory as f64) * 100.0;
                ui.horizontal(|ui| {
                    ui.label(format!("{}: ", key_type));
                    ui.label(format!(
                        "{} ({:.1}%)",
                        format_size(stats.total_memory),
                        percentage
                    ));
                });

                let bar_width = (percentage / 100.0) * 300.0;
                let color = type_color(key_type);

                let (rect, _) = ui
                    .allocate_exact_size(egui::vec2(bar_width as f32, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
            }
        });
        ui.add_space(10.0);
    }

    fn render_pie_chart_count(&self, ui: &mut egui::Ui, type_stats: &HashMap<String, TypeStats>) {
        ui.heading("Key Count by Type");
        ui.add_space(5.0);

        if type_stats.is_empty() {
            ui.label("No data available");
            return;
        }

        let total_count: usize = type_stats.values().map(|s| s.count).sum();
        if total_count == 0 {
            ui.label("No keys found");
            return;
        }

        let mut sorted_types: Vec<_> = type_stats.iter().collect();
        sorted_types.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(b.0)));

        ui.vertical(|ui| {
            for (key_type, stats) in sorted_types {
                let percentage = (stats.count as f64 / total_count as f64) * 100.0;
                ui.horizontal(|ui| {
                    ui.label(format!("{}: ", key_type));
                    ui.label(format!("{} keys ({:.1}%)", stats.count, percentage));
                });

                let bar_width = (percentage / 100.0) * 300.0;
                let color = type_color(key_type);

                let (rect, _) = ui
                    .allocate_exact_size(egui::vec2(bar_width as f32, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
            }
        });
        ui.add_space(10.0);
    }

    fn render_top_keys_table(&mut self, ui: &mut egui::Ui, analysis: &KeyAnalysis) {
        ui.heading("Top 20 Keys");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Sort by:");
            if ui
                .selectable_label(self.sort_mode == SortMode::BySize, "Memory Size")
                .clicked()
            {
                self.sort_mode = SortMode::BySize;
            }
            if ui
                .selectable_label(self.sort_mode == SortMode::ByLength, "Key Length")
                .clicked()
            {
                self.sort_mode = SortMode::ByLength;
            }
        });
        ui.add_space(5.0);

        if analysis.top_keys.is_empty() {
            ui.label("No keys analyzed yet");
            return;
        }

        let mut sorted_keys = analysis.top_keys.clone();
        match self.sort_mode {
            SortMode::BySize => {
                sorted_keys.sort_by(|a, b| b.size.cmp(&a.size));
            }
            SortMode::ByLength => {
                sorted_keys.sort_by(|a, b| b.name.len().cmp(&a.name.len()));
            }
        }

        use egui_extras::{Column, TableBuilder};

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .min_scrolled_height(400.0)
            .vscroll(true)
            .column(Column::auto()) // Rank
            .column(Column::remainder()) // Key name
            .column(Column::auto()) // Type
            .column(Column::auto()) // Size
            .column(Column::auto()) // Length
            .column(Column::auto()) // TTL
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("#");
                });
                header.col(|ui| {
                    ui.strong("Key");
                });
                header.col(|ui| {
                    ui.strong("Type");
                });
                header.col(|ui| {
                    ui.strong("Size");
                });
                header.col(|ui| {
                    ui.strong("Length");
                });
                header.col(|ui| {
                    ui.strong("TTL");
                });
            })
            .body(|mut body| {
                for (idx, key_info) in sorted_keys.iter().take(20).enumerate() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("{}", idx + 1));
                        });
                        row.col(|ui| {
                            let truncated = if key_info.name.len() > 50 {
                                format!("{}...", &key_info.name[..47])
                            } else {
                                key_info.name.clone()
                            };
                            ui.label(truncated);
                        });
                        row.col(|ui| {
                            ui.label(&key_info.key_type);
                        });
                        row.col(|ui| {
                            ui.label(format_size(key_info.size));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", key_info.name.len()));
                        });
                        row.col(|ui| {
                            let ttl_text = if key_info.ttl == -1 {
                                "No expiry".to_string()
                            } else if key_info.ttl == -2 {
                                "N/A".to_string()
                            } else {
                                format!("{}s", key_info.ttl)
                            };
                            ui.label(ttl_text);
                        });
                    });
                }
            });

        ui.add_space(10.0);
    }

    fn render_ttl_estimation(&self, ui: &mut egui::Ui, ttl_buckets: &TtlBuckets) {
        ui.heading("Space to be Freed by TTL");
        ui.add_space(5.0);

        let total_expiring = ttl_buckets.hour_1
            + ttl_buckets.hour_4
            + ttl_buckets.hour_24
            + ttl_buckets.hour_48
            + ttl_buckets.hour_72
            + ttl_buckets.week_1
            + ttl_buckets.month_1
            + ttl_buckets.month_plus;

        if total_expiring == 0 {
            ui.label("No keys with TTL found");
            return;
        }

        ui.vertical(|ui| {
            let buckets = [
                ("1 hour", ttl_buckets.hour_1),
                ("4 hours", ttl_buckets.hour_4),
                ("24 hours", ttl_buckets.hour_24),
                ("48 hours", ttl_buckets.hour_48),
                ("72 hours", ttl_buckets.hour_72),
                ("1 week", ttl_buckets.week_1),
                ("1 month", ttl_buckets.month_1),
                ("> 1 month", ttl_buckets.month_plus),
            ];

            for (label, size) in buckets {
                if size > 0 {
                    ui.horizontal(|ui| {
                        ui.label(format!("Next {}: ", label));
                        ui.label(format_size(size));
                    });
                }
            }
        });

        ui.add_space(10.0);
    }
}

impl Component for InsightsWindow {
    fn show(&mut self, ctx: &Context, state: &mut AppState) -> Result<(), Error> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Valkey Insight");
            ui.separator();

            if let Some(ref client) = state.valkey_client {
                let Some(last_update) = self.read_lock(&self.last_update, |lu| *lu) else {
                    return;
                };
                let elapsed = last_update.elapsed();

                let Some(stats_empty) = self.read_lock(&self.stats, |s| s.is_empty()) else {
                    return;
                };

                if stats_empty || elapsed > Duration::from_secs(2) {
                    self.fetch_stats(Arc::clone(client), ctx);
                }

                let Some(is_fetching) = self.read_lock(&self.is_fetching, |f| *f) else {
                    return;
                };

                if is_fetching {
                    ctx.request_repaint();
                }

                let Some(needs_analysis) = self.read_lock(&self.key_analysis, |analysis| {
                    analysis.last_analysis.is_none()
                        || analysis
                            .last_analysis
                            .is_some_and(|last| last.elapsed() > Duration::from_secs(60))
                }) else {
                    return;
                };

                let Some(is_analyzing) = self.read_lock(&self.is_analyzing, |a| *a) else {
                    return;
                };

                if needs_analysis && !is_analyzing {
                    self.analyze_keys(Arc::clone(client));
                }

                ui.horizontal(|ui| {
                    ui.label(format!("Last updated: {:.1}s ago", elapsed.as_secs_f32()));
                    if ui.button("🔄 Refresh Now").clicked() {
                        self.write_lock(&self.last_update, |lu| {
                            *lu = Instant::now() - Duration::from_secs(10)
                        });
                    }
                    ui.separator();
                    if ui
                        .button(if self.show_graphs {
                            "📊 Hide Graphs"
                        } else {
                            "📊 Show Graphs"
                        })
                        .clicked()
                    {
                        self.show_graphs = !self.show_graphs;
                    }
                    ui.separator();

                    let is_analyzing = self.read_lock(&self.is_analyzing, |a| *a).unwrap_or(false);
                    let button_text = if is_analyzing {
                        "🔄 Analyzing..."
                    } else {
                        "🔍 Analyze Keys"
                    };
                    if ui
                        .add_enabled(!is_analyzing, egui::Button::new(button_text))
                        .clicked()
                    {
                        self.analyze_keys(Arc::clone(client));
                    }

                    if let Some(last_analysis_time) =
                        self.read_lock(&self.key_analysis, |a| a.last_analysis)
                        && let Some(last) = last_analysis_time
                    {
                        ui.label(format!(
                            "(analyzed {:.0}s ago)",
                            last.elapsed().as_secs_f32()
                        ));
                    }
                });

                ui.separator();

                let Some(stats) = self.read_lock(&self.stats, |s| s.clone()) else {
                    return;
                };
                let Some(is_fetching) = self.read_lock(&self.is_fetching, |f| *f) else {
                    return;
                };
                let stats_empty = stats.is_empty();

                ui.horizontal(|ui| {
                    ui.label("📊 Real-time Metrics:");
                    ui.separator();

                    if let Some(clients) = stats.get("connected_clients") {
                        ui.label(format!("👥 Connected Clients: {}", clients));
                    } else if is_fetching && stats_empty {
                        ui.label("👥 Connected Clients: Loading...");
                    } else {
                        ui.label("👥 Connected Clients: N/A");
                    }

                    ui.separator();

                    if let Some(memory_str) = stats.get("used_memory") {
                        if let Ok(memory_bytes) = memory_str.parse::<u64>() {
                            ui.label(format!("💾 Used Space: {}", format_size(memory_bytes)));
                        } else if is_fetching && stats_empty {
                            ui.label("💾 Used Space: Loading...");
                        } else {
                            ui.label("💾 Used Space: N/A");
                        }
                    } else if is_fetching && stats_empty {
                        ui.label("💾 Used Space: Loading...");
                    } else {
                        ui.label("💾 Used Space: N/A");
                    }

                    ui.separator();

                    if let Some(dbsize) = stats.get("dbsize") {
                        ui.label(format!("🔑 DB Size: {} keys", dbsize));
                    } else if is_fetching && stats_empty {
                        ui.label("🔑 DB Size: Loading...");
                    } else {
                        ui.label("🔑 DB Size: N/A");
                    }
                });
                drop(stats);

                ui.separator();

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let analysis_clone = self
                            .read_lock(&self.key_analysis, |analysis| {
                                if analysis.last_analysis.is_some() {
                                    Some(analysis.clone())
                                } else {
                                    None
                                }
                            })
                            .flatten();

                        if let Some(analysis_data) = analysis_clone {
                            ui.heading("Key Analysis");
                            ui.separator();
                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    self.render_pie_chart_memory(ui, &analysis_data.type_stats);
                                });
                                ui.add_space(20.0);
                                ui.vertical(|ui| {
                                    self.render_pie_chart_count(ui, &analysis_data.type_stats);
                                });
                            });

                            ui.separator();
                            ui.add_space(10.0);

                            self.render_top_keys_table(ui, &analysis_data);

                            ui.separator();
                            ui.add_space(10.0);

                            self.render_ttl_estimation(ui, &analysis_data.ttl_buckets);

                            ui.separator();
                            ui.add_space(10.0);
                        } else {
                            let is_analyzing =
                                self.read_lock(&self.is_analyzing, |a| *a).unwrap_or(false);
                            if is_analyzing {
                                ui.heading("Key Analysis");
                                ui.separator();
                                ui.add_space(10.0);
                                ui.label("Analyzing keys... This may take a moment.");
                                ui.add_space(10.0);
                                ui.separator();
                            }
                        }
                    });
            } else {
                ui.label("No connection to Valkey server. Please connect first.");
            }
        });

        Ok(())
    }

    fn refresh(&mut self, _valkey_client: &Arc<ValkeyClient>) {
        self.write_lock(&self.last_update, |lu| {
            *lu = Instant::now() - Duration::from_secs(10)
        });
    }
}
