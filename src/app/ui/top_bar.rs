use crate::app::TempMailApp;

use eframe::egui::{self, ComboBox};

impl TempMailApp {
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui, ctx: &eframe::egui::Context) {
        self.email_heading(ui, ctx);
        self.email_selector(ui);
    }
}
impl TempMailApp {
    fn email_heading(&mut self, ui: &mut egui::Ui, ctx: &eframe::egui::Context) {
        ui.horizontal(|ui| {
            ui.heading([" ", &self.email()].join(""));
            if ui.button("copy").clicked() {
                ctx.copy_text(self.email());
            }
        });
    }
    fn email_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = ui.label("Name");
            ui.text_edit_singleline(&mut self.name)
                .labelled_by(label.id);
            if ui.button("↻").clicked() {
                self.name = Self::gen_random_name(10);
            }
            ui.label("@");
            ComboBox::from_label("")
                .selected_text(&self.domain)
                .show_ui(ui, |ui| {
                    for i in &self.domains {
                        ui.selectable_value(&mut self.domain, i.clone(), i);
                    }
                });
        });
    }
}
