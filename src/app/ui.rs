use crate::app::TempMailApp;
use eframe::egui::{self, CentralPanel, MenuBar, Spinner, TopBottomPanel, Widget, Window};

use crate::event_handler::Event;

pub mod email_list;
pub mod email_view;
pub mod top_bar;

impl TempMailApp {
    pub(super) fn app_ui(&mut self, ctx: &egui::Context) {
        self.images(ctx);
        self.menu_bar(ctx);
        self.body(ctx);
    }
}

impl TempMailApp {
    fn images(&mut self, ctx: &egui::Context) {
        for (index, (path, name)) in self.images.clone().iter().enumerate() {
            Window::new(name).show(ctx, |ui| {
                ui.image(path);
                if ui.button("close").clicked() {
                    self.images.remove(index);
                }
            });
        }
    }
    fn menu_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("menubar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ui.separator();
                if !self.name.is_empty()
                    && !self.emails.is_empty()
                    && ui.button("Delete all emails").clicked()
                {
                    self.send_event(Event::DeleteAllEmails(self.email()));
                    self.send_event(Event::FetchEmails(self.email()));
                }
            })
        });
    }
    fn body(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.set_width(ui.available_width());

                self.top_bar(ui, ctx);

                if self.viewed_email.is_some() && ui.button("Back to email list").clicked() {
                    self.viewed_email = None;
                    self.attachments = None;
                } else if self.viewed_email.is_none()
                    && !self.name.is_empty()
                    && ui.button("Fetch emails").clicked()
                {
                    self.send_event(Event::FetchEmails(self.email()));
                    self.send_event(Event::CountEmails(self.email()));
                }

                ui.spacing();

                if self.fetching {
                    ui.centered_and_justified(|ui| Spinner::new().size(50.).ui(ui));
                } else if let Some(email) = self.viewed_email.clone() {
                    self.email_view(ui, email);
                } else {
                    self.email_list(ui);
                };
            });
        });
    }
}
