use crate::app::TempMailApp;

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Separator, Widget};
use tmapi::Email;

use crate::event_handler::Event;

pub mod attachment_list;
impl TempMailApp {
    pub(super) fn email_view(&mut self, ui: &mut egui::Ui, email: Email) {
        let Email {
            id,
            from_address,
            to_address,
            subject,
            received_at,
            text_content,
            has_attachments,
            attachment_count,
            ..
        } = email;
        ScrollArea::vertical().show(ui, |ui| {
            let date = TempMailApp::get_date(received_at);
            ui.vertical(|ui| {
                ui.label(RichText::new(subject).size(40.).strong());
                Frame::group(ui.style())
                    .inner_margin(Margin::symmetric(16, 16))
                    .corner_radius(ui.style().visuals.menu_corner_radius)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.email_info(from_address, to_address, date, ui);
                        Separator::default().spacing(20.).ui(ui);
                        ui.horizontal_wrapped(|ui| ui.label(text_content.unwrap_or_default()));

                        if let Some(attachments) = self.attachments.clone() {
                            self.attachment_list(ui, &attachments);
                        } else if has_attachments
                            && ui.small_button(format!("📎{attachment_count}",)).clicked()
                        {
                            self.send_event(Event::GetAttachments(id));
                        }
                    });
            })
        });
    }
}
impl TempMailApp {
    fn email_info(
        &self,
        from_address: String,
        to_address: String,
        date: String,
        ui: &mut egui::Ui,
    ) {
        ui.label(format!("{:10} {from_address}", "From"));
        ui.label(format!("{:10} {to_address}", "To"));
        ui.spacing();
        ui.small(date);
    }
}
