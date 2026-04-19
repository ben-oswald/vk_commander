//! Application-wide design system and theming.
//!
//! Provides custom visuals, spacing, and color constants for a modern look.

use egui::{Color32, Context, CornerRadius, Shadow, Stroke, Style};

// =============================================================================
// Spacing Constants
// =============================================================================

/// Standard spacing unit (4px grid system).
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
pub const SPACE_XL: f32 = 24.0;

/// Sidebar dimensions.
pub const SIDEBAR_WIDTH: f32 = 72.0;
pub const SIDEBAR_ICON_SIZE: f32 = 20.0;
pub const SIDEBAR_BUTTON_SIZE: f32 = 48.0;

/// Standard corner radius for UI elements.
pub const CORNER_RADIUS_SM: u8 = 4;
pub const CORNER_RADIUS_MD: u8 = 6;
pub const CORNER_RADIUS_LG: u8 = 8;

/// Section header font size.
pub const SECTION_HEADER_SIZE: f32 = 13.0;

// =============================================================================
// Color Palette — Dark Mode
// =============================================================================

/// Dark mode background colors.
pub const DARK_BG_BASE: Color32 = Color32::from_rgb(30, 30, 46);
pub const DARK_BG_SURFACE: Color32 = Color32::from_rgb(36, 36, 54);
pub const DARK_BG_ELEVATED: Color32 = Color32::from_rgb(49, 50, 68);
pub const DARK_BG_OVERLAY: Color32 = Color32::from_rgb(59, 60, 78);

/// Dark mode text colors.
pub const DARK_TEXT_PRIMARY: Color32 = Color32::from_rgb(205, 214, 244);
pub const DARK_TEXT_SECONDARY: Color32 = Color32::from_rgb(147, 153, 178);
pub const DARK_TEXT_MUTED: Color32 = Color32::from_rgb(108, 112, 134);

/// Dark mode accent colors.
pub const DARK_ACCENT: Color32 = Color32::from_rgb(116, 199, 236);
pub const DARK_ACCENT_HOVER: Color32 = Color32::from_rgb(137, 220, 235);
pub const DARK_SELECTION: Color32 = Color32::from_rgb(49, 50, 68);

/// Dark mode semantic colors.
pub const DARK_SUCCESS: Color32 = Color32::from_rgb(166, 227, 161);
pub const DARK_ERROR: Color32 = Color32::from_rgb(243, 139, 168);
pub const DARK_WARNING: Color32 = Color32::from_rgb(249, 226, 175);
pub const DARK_INFO: Color32 = Color32::from_rgb(116, 199, 236);

/// Dark mode borders.
pub const DARK_BORDER: Color32 = Color32::from_rgb(65, 66, 85);
pub const DARK_BORDER_SUBTLE: Color32 = Color32::from_rgb(55, 56, 74);

/// Dark mode sidebar.
pub const DARK_SIDEBAR_BG: Color32 = Color32::from_rgb(24, 24, 38);
pub const DARK_SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(49, 50, 68);
pub const DARK_SIDEBAR_HOVER: Color32 = Color32::from_rgb(42, 42, 60);

// =============================================================================
// Color Palette — Light Mode
// =============================================================================

/// Light mode background colors.
pub const LIGHT_BG_BASE: Color32 = Color32::from_rgb(239, 241, 245);
pub const LIGHT_BG_SURFACE: Color32 = Color32::from_rgb(241, 243, 247);
pub const LIGHT_BG_ELEVATED: Color32 = Color32::from_rgb(233, 236, 242);
pub const LIGHT_BG_OVERLAY: Color32 = Color32::from_rgb(218, 222, 230);

/// Light mode text colors.
pub const LIGHT_TEXT_PRIMARY: Color32 = Color32::from_rgb(76, 79, 105);
pub const LIGHT_TEXT_SECONDARY: Color32 = Color32::from_rgb(108, 111, 133);
pub const LIGHT_TEXT_MUTED: Color32 = Color32::from_rgb(140, 143, 161);

/// Light mode accent colors.
pub const LIGHT_ACCENT: Color32 = Color32::from_rgb(30, 102, 245);
pub const LIGHT_ACCENT_HOVER: Color32 = Color32::from_rgb(24, 85, 210);
pub const LIGHT_SELECTION: Color32 = Color32::from_rgb(204, 208, 218);

/// Light mode semantic colors.
pub const LIGHT_SUCCESS: Color32 = Color32::from_rgb(64, 160, 43);
pub const LIGHT_ERROR: Color32 = Color32::from_rgb(210, 15, 57);
pub const LIGHT_WARNING: Color32 = Color32::from_rgb(223, 142, 29);
pub const LIGHT_INFO: Color32 = Color32::from_rgb(32, 159, 181);

/// Light mode borders.
pub const LIGHT_BORDER: Color32 = Color32::from_rgb(188, 192, 204);
pub const LIGHT_BORDER_SUBTLE: Color32 = Color32::from_rgb(204, 208, 218);

/// Light mode sidebar.
pub const LIGHT_SIDEBAR_BG: Color32 = Color32::from_rgb(233, 236, 242);
pub const LIGHT_SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(204, 208, 218);
pub const LIGHT_SIDEBAR_HOVER: Color32 = Color32::from_rgb(210, 215, 225);

// =============================================================================
// Chart Colors (shared)
// =============================================================================

/// Returns a color for the given Valkey key type, suitable for charts.
pub fn type_color(key_type: &str) -> Color32 {
    match key_type {
        "string" => Color32::from_rgb(243, 139, 168),
        "hash" => Color32::from_rgb(116, 199, 236),
        "list" => Color32::from_rgb(249, 226, 175),
        "set" => Color32::from_rgb(148, 226, 213),
        "zset" => Color32::from_rgb(180, 149, 255),
        _ => Color32::from_rgb(147, 153, 178),
    }
}

// =============================================================================
// Theme Application
// =============================================================================

/// Applies the full custom visual style to the egui context.
///
/// Call this once on the first frame after setting the base theme (light/dark).
pub fn apply_custom_visuals(ctx: &Context) {
    let is_dark = ctx.style().visuals.dark_mode;

    let mut style = (*ctx.style()).clone();

    // Only adjust scroll bar width — don't override other spacing
    // to avoid breaking existing layouts that depend on defaults.
    style.spacing.scroll.bar_width = 6.0;

    if is_dark {
        apply_dark_visuals(&mut style);
    } else {
        apply_light_visuals(&mut style);
    }

    ctx.set_style(style);
}

fn apply_dark_visuals(style: &mut Style) {
    let v = &mut style.visuals;
    let cr = CornerRadius::same(CORNER_RADIUS_MD);

    v.dark_mode = true;
    v.override_text_color = None;
    v.panel_fill = DARK_BG_SURFACE;
    v.window_fill = DARK_BG_SURFACE;
    v.extreme_bg_color = DARK_BG_BASE;
    v.faint_bg_color = Color32::from_rgba_premultiplied(255, 255, 255, 12);
    v.code_bg_color = DARK_BG_ELEVATED;

    v.window_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 60),
    };
    v.window_corner_radius = CornerRadius::same(CORNER_RADIUS_LG);
    v.window_stroke = Stroke::new(1.0, DARK_BORDER);
    v.popup_shadow = Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 40),
    };
    v.menu_corner_radius = CornerRadius::same(CORNER_RADIUS_MD);

    v.selection.bg_fill = Color32::from_rgba_premultiplied(116, 199, 236, 40);
    v.selection.stroke = Stroke::new(1.0, DARK_ACCENT);

    // Hyperlink
    v.hyperlink_color = DARK_ACCENT;

    // Widgets — noninteractive
    v.widgets.noninteractive.bg_fill = DARK_BG_ELEVATED;
    v.widgets.noninteractive.weak_bg_fill = DARK_BG_ELEVATED;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, DARK_BORDER_SUBTLE);
    v.widgets.noninteractive.corner_radius = cr;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, DARK_TEXT_SECONDARY);

    // Widgets — inactive (buttons, combo boxes, etc.)
    v.widgets.inactive.bg_fill = DARK_BG_ELEVATED;
    v.widgets.inactive.weak_bg_fill = DARK_BG_ELEVATED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, DARK_BORDER);
    v.widgets.inactive.corner_radius = cr;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, DARK_TEXT_PRIMARY);

    // Widgets — hovered
    v.widgets.hovered.bg_fill = DARK_BG_OVERLAY;
    v.widgets.hovered.weak_bg_fill = DARK_BG_OVERLAY;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, DARK_ACCENT);
    v.widgets.hovered.corner_radius = cr;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, DARK_TEXT_PRIMARY);

    // Widgets — active (clicked)
    v.widgets.active.bg_fill = DARK_ACCENT;
    v.widgets.active.weak_bg_fill = Color32::from_rgba_premultiplied(116, 199, 236, 30);
    v.widgets.active.bg_stroke = Stroke::new(1.0, DARK_ACCENT);
    v.widgets.active.corner_radius = cr;
    v.widgets.active.fg_stroke = Stroke::new(1.0, DARK_BG_BASE);

    // Widgets — open (expanded combo box, etc.)
    v.widgets.open.bg_fill = DARK_BG_OVERLAY;
    v.widgets.open.weak_bg_fill = DARK_BG_OVERLAY;
    v.widgets.open.bg_stroke = Stroke::new(1.0, DARK_ACCENT);
    v.widgets.open.corner_radius = cr;
    v.widgets.open.fg_stroke = Stroke::new(1.0, DARK_TEXT_PRIMARY);

    // Striped table rows
    v.striped = true;

    // Text cursor
    v.text_cursor.stroke = Stroke::new(2.0, DARK_ACCENT);
}

fn apply_light_visuals(style: &mut Style) {
    let v = &mut style.visuals;
    let cr = CornerRadius::same(CORNER_RADIUS_MD);

    v.dark_mode = false;
    v.override_text_color = None;
    v.panel_fill = LIGHT_BG_SURFACE;
    v.window_fill = LIGHT_BG_SURFACE;
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::from_rgba_premultiplied(0, 0, 0, 8);
    v.code_bg_color = LIGHT_BG_ELEVATED;

    v.window_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 20),
    };
    v.window_corner_radius = CornerRadius::same(CORNER_RADIUS_LG);
    v.window_stroke = Stroke::new(1.0, LIGHT_BORDER);
    v.popup_shadow = Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 15),
    };
    v.menu_corner_radius = CornerRadius::same(CORNER_RADIUS_MD);

    v.selection.bg_fill = Color32::from_rgba_premultiplied(30, 102, 245, 30);
    v.selection.stroke = Stroke::new(1.0, LIGHT_ACCENT);

    // Hyperlink
    v.hyperlink_color = LIGHT_ACCENT;

    // Widgets — noninteractive
    v.widgets.noninteractive.bg_fill = LIGHT_BG_ELEVATED;
    v.widgets.noninteractive.weak_bg_fill = LIGHT_BG_ELEVATED;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LIGHT_BORDER_SUBTLE);
    v.widgets.noninteractive.corner_radius = cr;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, LIGHT_TEXT_SECONDARY);

    // Widgets — inactive
    v.widgets.inactive.bg_fill = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Color32::WHITE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LIGHT_BORDER);
    v.widgets.inactive.corner_radius = cr;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, LIGHT_TEXT_PRIMARY);

    // Widgets — hovered
    v.widgets.hovered.bg_fill = LIGHT_BG_OVERLAY;
    v.widgets.hovered.weak_bg_fill = LIGHT_BG_OVERLAY;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, LIGHT_ACCENT);
    v.widgets.hovered.corner_radius = cr;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, LIGHT_TEXT_PRIMARY);

    // Widgets — active
    v.widgets.active.bg_fill = LIGHT_ACCENT;
    v.widgets.active.weak_bg_fill = Color32::from_rgba_premultiplied(30, 102, 245, 25);
    v.widgets.active.bg_stroke = Stroke::new(1.0, LIGHT_ACCENT);
    v.widgets.active.corner_radius = cr;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Widgets — open
    v.widgets.open.bg_fill = LIGHT_BG_OVERLAY;
    v.widgets.open.weak_bg_fill = LIGHT_BG_OVERLAY;
    v.widgets.open.bg_stroke = Stroke::new(1.0, LIGHT_ACCENT);
    v.widgets.open.corner_radius = cr;
    v.widgets.open.fg_stroke = Stroke::new(1.0, LIGHT_TEXT_PRIMARY);

    // Striped
    v.striped = true;

    // Text cursor
    v.text_cursor.stroke = Stroke::new(2.0, LIGHT_ACCENT);
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Returns the accent color for the current theme.
pub fn accent_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_ACCENT
    } else {
        LIGHT_ACCENT
    }
}

/// Returns the muted text color for the current theme.
pub fn muted_text_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_TEXT_MUTED
    } else {
        LIGHT_TEXT_MUTED
    }
}

/// Returns the sidebar background color for the current theme.
pub fn sidebar_bg(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_SIDEBAR_BG
    } else {
        LIGHT_SIDEBAR_BG
    }
}

/// Returns the sidebar active item color for the current theme.
pub fn sidebar_active_bg(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_SIDEBAR_ACTIVE
    } else {
        LIGHT_SIDEBAR_ACTIVE
    }
}

/// Returns the sidebar hover item color for the current theme.
pub fn sidebar_hover_bg(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_SIDEBAR_HOVER
    } else {
        LIGHT_SIDEBAR_HOVER
    }
}

/// Returns the success color for the current theme.
pub fn success_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_SUCCESS
    } else {
        LIGHT_SUCCESS
    }
}

/// Returns the error color for the current theme.
pub fn error_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_ERROR
    } else {
        LIGHT_ERROR
    }
}

/// Returns the warning color for the current theme.
pub fn warning_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_WARNING
    } else {
        LIGHT_WARNING
    }
}

/// Returns the info color for the current theme.
pub fn info_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        DARK_INFO
    } else {
        LIGHT_INFO
    }
}

/// Creates a styled section header label.
pub fn section_header(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong().size(SECTION_HEADER_SIZE)
}

/// Renders a styled card frame (elevated surface with border).
pub fn card_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark {
            DARK_BG_ELEVATED
        } else {
            Color32::WHITE
        })
        .stroke(Stroke::new(
            1.0,
            if dark {
                DARK_BORDER_SUBTLE
            } else {
                LIGHT_BORDER_SUBTLE
            },
        ))
        .corner_radius(CornerRadius::same(CORNER_RADIUS_MD))
        .inner_margin(egui::Margin::same(SPACE_MD as i8))
}

/// Renders a subtle section frame (slightly elevated).
pub fn section_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark { DARK_BG_BASE } else { LIGHT_BG_BASE })
        .stroke(Stroke::new(
            1.0,
            if dark {
                DARK_BORDER_SUBTLE
            } else {
                LIGHT_BORDER_SUBTLE
            },
        ))
        .corner_radius(CornerRadius::same(CORNER_RADIUS_MD))
        .inner_margin(egui::Margin::same(SPACE_SM as i8))
}

/// Returns a styled status badge with the given text and background color.
pub fn status_badge(text: &str, color: Color32) -> egui::RichText {
    egui::RichText::new(text).color(color).strong().size(11.0)
}
