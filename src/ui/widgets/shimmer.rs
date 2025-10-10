use egui::{Color32, Rect, Response, Sense, Ui, Vec2};
use std::f32::consts::PI;

pub struct Shimmer {
    width: f32,
    height: f32,
}

impl Shimmer {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn show(&self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.width, self.height), Sense::hover());

        if ui.is_rect_visible(rect) {
            self.paint_shimmer(ui, rect);
        }

        response
    }

    fn paint_shimmer(&self, ui: &mut Ui, rect: Rect) {
        let time = ui.input(|i| i.time);

        let base_color = if ui.visuals().dark_mode {
            Color32::from_gray(45)
        } else {
            Color32::from_gray(230)
        };

        let shimmer_color = if ui.visuals().dark_mode {
            Color32::from_gray(65)
        } else {
            Color32::from_gray(250)
        };

        ui.painter().rect_filled(
            rect,
            ui.visuals().widgets.noninteractive.corner_radius,
            base_color,
        );

        let cycle_duration = 1.5;
        let progress = ((time % cycle_duration) / cycle_duration) as f32;

        let shimmer_width = rect.width() * 0.3;
        let shimmer_start_x =
            rect.min.x + (rect.width() + shimmer_width) * progress - shimmer_width;

        let steps = 20;
        for i in 0..steps {
            let step_progress = i as f32 / steps as f32;
            let x_offset = shimmer_start_x + shimmer_width * step_progress;

            let alpha_factor = (step_progress * PI).sin();
            let alpha = (alpha_factor * 0.6 * 255.0) as u8;

            let mut color = shimmer_color;
            color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

            let step_rect = Rect::from_min_max(
                [x_offset, rect.min.y].into(),
                [x_offset + shimmer_width / steps as f32, rect.max.y].into(),
            );

            if step_rect.intersects(rect) {
                ui.painter()
                    .rect_filled(step_rect.intersect(rect), 0.0, color);
            }
        }

        ui.ctx().request_repaint();
    }
}

pub fn shimmer(ui: &mut Ui, width: f32) -> Response {
    Shimmer::new(width, ui.text_style_height(&egui::TextStyle::Body)).show(ui)
}

pub fn shimmer_inline(ui: &mut Ui, width: f32) -> Response {
    let height = ui.text_style_height(&egui::TextStyle::Body);
    Shimmer::new(width, height * 0.8).show(ui)
}

pub fn shimmer_text(ui: &mut Ui, text: &str) -> Response {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, Color32::WHITE);
    let width = galley.rect.width();
    let height = ui.text_style_height(&egui::TextStyle::Body);
    Shimmer::new(width.max(40.0), height * 0.6).show(ui)
}
