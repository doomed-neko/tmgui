use eframe::egui::{Context, FontId, TextStyle};

pub fn set_styles(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(30., eframe::egui::FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(18., eframe::egui::FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(22., eframe::egui::FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(14., eframe::egui::FontFamily::Monospace),
        ),
        (
            TextStyle::Monospace,
            FontId::new(16., eframe::egui::FontFamily::Monospace),
        ),
    ]
    .into();
    ctx.set_style(style);
}
