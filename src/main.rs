use std::sync::{Arc, Mutex};

use chrono::TimeZone;
use eframe::{
    App,
    egui::{
        self, CentralPanel, ComboBox, Context, FontId, Frame, ScrollArea, Separator, Spinner,
        Stroke, TextStyle, TopBottomPanel, ViewportBuilder, Widget,
    },
};
use futures::executor::block_on;
use rand::Rng;
use tmapi::{Client, Email};

struct TmApp {
    domain: String,
    domains: Vec<String>,
    name: String,
    emails: Arc<Mutex<Vec<Email>>>,
    fetching_emails: Arc<Mutex<bool>>,
    viewed_email: Option<Email>,
}

impl Default for TmApp {
    fn default() -> Self {
        let client = Client::new("email@example.com").unwrap();
        let domains = futures::executor::block_on(client.get_domains()).unwrap();
        let name = gen_random_name(10);

        Self {
            domains,
            domain: "iusearch.lol".into(),
            name,
            emails: Arc::new(Mutex::new(vec![])),
            viewed_email: None,
            fetching_emails: Default::default(),
        }
    }
}

fn gen_random_name(len: usize) -> String {
    rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(len)
        .map(|x| x as char)
        .collect::<String>()
}

impl TmApp {
    pub fn email(&self) -> String {
        format!("{}@{}", self.name, self.domain)
    }
    pub fn client(&self) -> Client {
        Client::new(self.email()).unwrap()
    }

    fn fetch_emails(&mut self) {
        let client = self.client();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let status = Arc::clone(&self.fetching_emails);
        tokio::spawn(async move {
            *status.lock().unwrap() = true;
            match client.get_emails(50, 0).await {
                Ok(emails) => {
                    let _ = tx.send(emails).await;
                }
                Err(err) => {
                    eprintln!("Failed to fetch emails: {err:?}");
                }
            }
        });
        let emails_arc = Arc::clone(&self.emails);
        let status = Arc::clone(&self.fetching_emails);
        tokio::spawn(async move {
            while let Some(emails) = rx.recv().await {
                *emails_arc.lock().unwrap() = emails;
                *status.lock().unwrap() = false;
            }
        });
    }
    fn top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Actions", |ui| {
                    if !self.name.is_empty()
                        && !self.emails.lock().unwrap().is_empty()
                        && ui.button("Delete all emails").clicked()
                    {
                        block_on(self.client().delete_all_emails()).ok();
                        self.fetch_emails();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                })
            })
        });
    }

    fn delete<S>(&mut self, id: S)
    where
        S: Into<String>,
    {
        let _ = block_on(self.client().delete_inbox(id.into()));
        self.fetch_emails();
    }

    fn get_email<S>(&self, id: S) -> Email
    where
        S: Into<String>,
    {
        block_on(self.client().get_inbox(id)).unwrap()
    }

    fn email_list(&mut self, _ctx: &Context, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            let emails = self.emails.clone();
            let emails = emails.lock().unwrap();
            if emails.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.heading("No emails are here yet");
                });
                return;
            }
            for Email {
                id,
                from_address,
                to_address,
                subject,
                received_at,
                ..
            } in emails.clone()
            {
                let date = chrono::Local
                    .timestamp_opt(received_at, 0)
                    .unwrap()
                    .to_string();
                Frame::new().stroke(Stroke::default()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(&subject);
                            if ui.small_button("🗑").clicked() {
                                self.delete(&id);
                            }
                            if ui.small_button("📩").clicked() {
                                let email = self.get_email(&id);
                                self.viewed_email = Some(email);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.small(from_address);
                            ui.small("→");
                            ui.small(to_address);
                        });
                        ui.small(date);
                    })
                });
                ui.separator();
            }
        });
    }

    fn email_edit_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = ui.label("Name");
            ui.text_edit_singleline(&mut self.name)
                .labelled_by(label.id);
            if ui.button("↻").clicked() {
                self.name = gen_random_name(10);
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

impl App for TmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);

        self.top_bar(ctx);

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading(self.email());
                self.email_edit_view(ui);

                if self.viewed_email.is_some() && ui.button("Back to email list").clicked() {
                    self.viewed_email = None;
                }
                if self.viewed_email.is_none()
                    && !self.name.is_empty()
                    && ui.button("Fetch emails").clicked()
                {
                    self.fetch_emails();
                }

                ui.spacing();

                if *self.fetching_emails.lock().unwrap() {
                    ui.centered_and_justified(|ui| Spinner::new().size(50.).ui(ui));
                } else if let Some(email) = self.viewed_email.clone() {
                    email_view(ui, email);
                } else {
                    self.email_list(ctx, ui);
                }
            });
        });
    }
}

fn email_view(ui: &mut egui::Ui, email: Email) {
    let Email {
        from_address,
        to_address,
        subject,
        received_at,
        text_content,
        ..
    } = email;
    ScrollArea::vertical().show(ui, |ui| {
        let date = chrono::Local
            .timestamp_opt(received_at, 0)
            .unwrap()
            .to_string();
        ui.vertical(|ui| {
            ui.heading(format!("subject: {subject}"));
            ui.heading(format!("time: {date}"));
            ui.horizontal(|ui| {
                ui.small(from_address);
                ui.label("→");
                ui.small(to_address);
            });
            Separator::default().spacing(5.).ui(ui);
            ui.horizontal_wrapped(|ui| ui.label(text_content.unwrap_or_default()));
        })
    });
}

#[tokio::main]
async fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_app_id("adenosine.tmgui"),
        ..Default::default()
    };

    eframe::run_native("TMApi", opts, Box::new(|_| Ok(Box::<TmApp>::default())))
}

fn set_styles(ctx: &Context) {
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
