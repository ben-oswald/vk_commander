use crate::state::{BannerKind, BannerParams};
use eframe::egui;

pub struct Banner {
    id: String,
    header: String,
    message: String,
    kind: BannerKind,
    created_at: std::time::Instant,
    duration: std::time::Duration,
    paused_since: Option<std::time::Instant>,
    paused_accumulated: std::time::Duration,
    request: Option<String>,
}

impl Banner {
    pub fn from_params(id: String, params: &BannerParams) -> Self {
        Self {
            id,
            header: params.header.clone(),
            message: params.message.clone(),
            kind: params.kind,
            created_at: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(params.duration_ms),
            paused_since: None,
            paused_accumulated: std::time::Duration::ZERO,
            request: params.request.clone(),
        }
    }

    pub fn still_visible(&self, now: std::time::Instant, grace_ms: u64) -> bool {
        self.active_elapsed(now) < self.duration + std::time::Duration::from_millis(grace_ms)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        now: std::time::Instant,
        y_offset: f32,
        margin: f32,
        width: f32,
    ) -> (f32, bool) {
        const FADE_IN_MS: u64 = 200;
        const FADE_OUT_MS: u64 = 300;

        let id = egui::Id::new(format!("banner_{}", self.id));
        egui::Area::new(id)
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                egui::vec2(-margin, -margin - y_offset),
            )
            .movable(false)
            .show(ctx, |ui| {
                ui.set_max_width(width);

                let fade = self.fade_factor(now, FADE_IN_MS, FADE_OUT_MS);
                let bg = self.background_color(fade);

                let mut stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
                let sa = (120.0 * fade).round().clamp(0.0, 255.0) as u8;
                stroke.color = egui::Color32::from_rgba_premultiplied(0, 0, 0, sa);

                let frame = egui::Frame::new()
                    .fill(bg)
                    .stroke(stroke)
                    .corner_radius(ui.style().visuals.widgets.noninteractive.corner_radius)
                    .inner_margin(egui::Margin::same(0));

                let inner = frame.show(ui, |ui| {
                    // Progress bar
                    let remaining_frac = self.progress_remaining_fraction(now);
                    let bar_height = 3.0;
                    let available_width = ui.available_width();
                    let (track_rect, _) = ui.allocate_exact_size(
                        egui::vec2(available_width, bar_height),
                        egui::Sense::hover(),
                    );
                    let rounding = bar_height / 2.0;
                    let track_color = Self::premul(255, 255, 255, 40, fade);
                    ui.painter().rect_filled(track_rect, rounding, track_color);

                    let fill_color = match self.kind {
                        BannerKind::Info => Self::premul(150, 210, 255, 220, fade),
                        BannerKind::Error => Self::premul(255, 120, 120, 220, fade),
                        BannerKind::Success => Self::premul(120, 255, 150, 220, fade),
                    };
                    let fill_width = (track_rect.width() * remaining_frac).max(0.0);
                    if fill_width > 0.0 {
                        let fill_rect = egui::Rect::from_min_size(
                            track_rect.min,
                            egui::vec2(fill_width, track_rect.height()),
                        );
                        ui.painter().rect_filled(fill_rect, rounding, fill_color);
                    }

                    egui::Frame::NONE
                        .inner_margin(egui::Margin {
                            left: 10,
                            right: 10,
                            top: 6,
                            bottom: 10,
                        })
                        .show(ui, |ui| {
                            let (text_color_head, text_color_body) = Self::text_colors(fade);
                            ui.horizontal(|ui| {
                                ui.heading(
                                    egui::RichText::new(self.header.clone())
                                        .color(text_color_head)
                                        .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(self.formatted_message())
                                    .color(text_color_body),
                            );
                        })
                });

                let rect = inner.response.rect;
                let interact_id = ui.id().with(format!("{}_interact", self.id));
                let resp = ui.interact(rect, interact_id, egui::Sense::click());

                let mut open = true;
                if resp.clicked() {
                    open = false;
                }

                let hovered = resp.hovered();
                if hovered {
                    if self.paused_since.is_none() {
                        self.paused_since = Some(now);
                    }
                } else if let Some(ps) = self.paused_since {
                    self.paused_accumulated = self
                        .paused_accumulated
                        .saturating_add(now.saturating_duration_since(ps));
                    self.paused_since = None;
                }

                let used_height = inner.response.rect.height();
                (used_height, !open)
            })
            .inner
    }

    fn formatted_message(&self) -> String {
        match &self.request {
            Some(req) => format!("{}: {}", req, self.message),
            None => self.message.clone(),
        }
    }

    fn active_elapsed(&self, now: std::time::Instant) -> std::time::Duration {
        let mut paused_acc = self.paused_accumulated;
        if let Some(ps) = self.paused_since {
            paused_acc = paused_acc.saturating_add(now.saturating_duration_since(ps));
        }
        now.saturating_duration_since(self.created_at)
            .saturating_sub(paused_acc)
    }

    fn fade_factor(&self, now: std::time::Instant, fade_in_ms: u64, fade_out_ms: u64) -> f32 {
        let elapsed_ms = self.active_elapsed(now).as_millis() as u64;
        let total_ms = self.duration.as_millis() as u64;
        if elapsed_ms < fade_in_ms {
            elapsed_ms as f32 / fade_in_ms as f32
        } else if elapsed_ms > total_ms {
            let over = elapsed_ms - total_ms;
            if over >= fade_out_ms {
                0.0
            } else {
                1.0 - (over as f32 / fade_out_ms as f32)
            }
        } else {
            1.0
        }
    }

    fn premul(r: u8, g: u8, b: u8, a: u8, f: f32) -> egui::Color32 {
        let a_f = ((a as f32) / 255.0) * f;
        let pr = ((r as f32) * a_f).round().clamp(0.0, 255.0) as u8;
        let pg = ((g as f32) * a_f).round().clamp(0.0, 255.0) as u8;
        let pb = ((b as f32) * a_f).round().clamp(0.0, 255.0) as u8;
        let pa = (a_f * 255.0).round().clamp(0.0, 255.0) as u8;
        egui::Color32::from_rgba_premultiplied(pr, pg, pb, pa)
    }

    fn background_color(&self, fade: f32) -> egui::Color32 {
        match self.kind {
            BannerKind::Info => Self::premul(40, 100, 200, 235, fade),
            BannerKind::Error => Self::premul(160, 30, 30, 235, fade),
            BannerKind::Success => Self::premul(40, 140, 80, 235, fade),
        }
    }

    fn progress_remaining_fraction(&self, now: std::time::Instant) -> f32 {
        let total = self.duration.as_secs_f32().max(0.000_1);
        let active_elapsed = self.active_elapsed(now).as_secs_f32();
        let remaining = 1.0 - (active_elapsed / total);
        remaining.clamp(0.0, 1.0)
    }

    fn text_colors(fade: f32) -> (egui::Color32, egui::Color32) {
        let head = egui::Color32::from_rgba_premultiplied(
            255,
            255,
            255,
            (255.0 * fade).round().clamp(0.0, 255.0) as u8,
        );
        let body = egui::Color32::from_rgba_premultiplied(
            240,
            240,
            240,
            (245.0 * fade).round().clamp(0.0, 255.0) as u8,
        );
        (head, body)
    }
}
