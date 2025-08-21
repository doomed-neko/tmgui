use eframe::egui::{self, Frame, Margin};
use std::fmt::{Display, Write};
use tmapi::Attachment;

use crate::app::TempMailApp;

impl TempMailApp {
    pub(super) fn attachment_list(&mut self, ui: &mut egui::Ui, attachments: &[Attachment]) {
        ui.vertical(|ui| {
            for attachments in attachments.chunks(3) {
                for attachment in attachments {
                    self.attachment_tile(ui, attachment);
                }
                ui.end_row();
            }
        });
    }
}

impl TempMailApp {
    fn attachment_tile(&mut self, ui: &mut egui::Ui, attachment: &Attachment) {
        Frame::group(ui.style())
            .inner_margin(Margin::symmetric(5, 5))
            .corner_radius(ui.style().visuals.menu_corner_radius)
            .show(ui, |ui| {
                let file_ext = attachment.filename.split('.').next_back();
                let is_img = ["jpg", "png"].contains(&file_ext.unwrap_or_default());
                let file_icon = if is_img { "🖼" } else { "📄" };
                ui.horizontal(|ui| {
                    ui.label(format!("{}{}", file_icon, attachment.filename));
                    ui.separator();
                    ui.label(UnitSize(attachment.size).to_string());
                    ui.separator();
                    let api_url = "https://api.barid.site";
                    let path = format!("{api_url}/attachments/{}", attachment.id);
                    if ui.button("↓").clicked() {
                        open::that(&path).ok();
                    }
                    let view_info = (path, attachment.filename.clone());
                    if is_img && ui.button("open").clicked() && !self.images.contains(&view_info) {
                        self.images.push(view_info);
                    }
                })
            });
    }
}

struct UnitSize(pub u64);
impl Display for UnitSize {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = self.0 as f64;
        const KILO: f64 = 1024.;
        const MEGA: f64 = KILO * KILO;
        const GIGA: f64 = KILO * MEGA;
        match val {
            0. => fmt.write_char('0'),
            val if val < KILO => write!(fmt, "{}B", val as u64),
            val if val < MEGA => write!(fmt, "{:.2}KB", val / KILO),
            val if val < GIGA => write!(fmt, "{:.2}MB", val / MEGA),
            val => write!(fmt, "{:.2}GB", val / GIGA),
        }
    }
}
