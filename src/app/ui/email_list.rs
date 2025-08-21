use crate::app::TempMailApp;

use eframe::egui::{self, Frame, ScrollArea, Stroke};
use tmapi::Email;

use crate::event_handler::Event;

impl TempMailApp {
    pub(super) fn email_list(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            if self.emails.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.heading("No emails are here yet");
                });
                return;
            }
            for (index, email) in self.emails.clone().into_iter().enumerate() {
                self.email_tile(ui, index, email);
            }
            if self.email_count as usize > self.emails.len() && ui.button("Load more").clicked() {
                self.send_event(Event::FetchMoreEmails(self.email(), self.current_offset));
                self.current_offset += 1;
            };
        });
    }
}
impl TempMailApp {
    fn email_tile(&mut self, ui: &mut egui::Ui, index: usize, email: Email) {
        let Email {
            id,
            from_address,
            subject,
            received_at,
            has_attachments,
            attachment_count,
            ..
        } = email;
        let date = Self::get_date(received_at);
        Frame::new().stroke(Stroke::default()).show(ui, |ui| {
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::relative(0.9))
                .size(egui_extras::Size::remainder());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if has_attachments {
                        ui.label(format!("📎{attachment_count}"));
                    }
                    self.trash_button(index, &id, ui);
                    self.open_button(id, ui);
                });
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&subject);
                    });
                    ui.small(from_address);
                    ui.small(date);
                });
            });
        });
        ui.separator();
    }

    fn open_button(&mut self, id: String, ui: &mut egui::Ui) {
        if ui.small_button("📩").clicked() {
            self.send_event(Event::FetchEmail(id.clone()));
        }
    }

    fn trash_button(&mut self, index: usize, id: &str, ui: &mut egui::Ui) {
        if ui.small_button("🗑").clicked() {
            self.send_event(Event::DeleteEmail(id.to_owned(), index));
        }
    }
}
