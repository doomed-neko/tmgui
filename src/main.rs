use std::sync::{Arc, Mutex};

use chrono::{Datelike, Month, TimeZone, Timelike};
use eframe::{
    App,
    egui::{
        self, CentralPanel, ComboBox, Context, FontId, Frame, Margin, RichText, ScrollArea,
        Separator, Spinner, Stroke, TextStyle, TopBottomPanel, ViewportBuilder, Widget,
    },
};
use futures::executor::block_on;
use rand::Rng;
use tmapi::{Client, Email};

const EMAIL_FETCHING_LIMIT: u8 = 50;

struct TmApp {
    domain: String,
    domains: Vec<String>,
    name: String,
    emails: Arc<Mutex<Vec<Email>>>,
    fetching_emails: Arc<Mutex<bool>>,
    viewed_email: Arc<Mutex<Option<Email>>>,
    email_count: Arc<Mutex<u32>>,
    current_offset: Arc<Mutex<u32>>,
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
            viewed_email: Default::default(),
            fetching_emails: Default::default(),
            email_count: Default::default(),
            current_offset: Default::default(),
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

    fn top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Actions", |ui| {
                    if !self.name.is_empty()
                        && !self.emails.lock().unwrap().is_empty()
                        && ui.button("Delete all emails").clicked()
                    {
                        block_on(self.client().delete_all_emails()).ok();
                        self.fetch_emails(false);
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
        self.fetch_emails(false);
    }

    fn fetch_emails(&mut self, extend: bool) {
        let client = self.client();
        let mut offset = *self.current_offset.lock().unwrap();
        if extend {
            offset += 1;
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let status = Arc::clone(&self.fetching_emails);
        tokio::spawn(async move {
            *status.lock().unwrap() = true;
            match client.get_emails(EMAIL_FETCHING_LIMIT, offset).await {
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
                if extend {
                    emails_arc.lock().unwrap().extend_from_slice(&emails);
                } else {
                    *emails_arc.lock().unwrap() = emails;
                }
                *status.lock().unwrap() = false;
            }
        });
    }
    fn count_emails(&mut self) {
        let client = self.client();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            match client.email_count().await {
                Ok(emails) => {
                    let _ = tx.send(emails).await;
                }
                Err(err) => {
                    eprintln!("Failed to fetch emails: {err:?}");
                }
            }
        });

        let emails_arc = Arc::clone(&self.email_count);
        tokio::spawn(async move {
            while let Some(emails) = rx.recv().await {
                *emails_arc.lock().unwrap() = emails;
            }
        });
    }
    fn get_email<S>(&self, id: S)
    where
        S: Into<String>,
    {
        let id = id.into();
        let client = self.client();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let status = Arc::clone(&self.fetching_emails);
        tokio::spawn(async move {
            *status.lock().unwrap() = true;
            match client.get_inbox(id).await {
                Ok(emails) => {
                    let _ = tx.send(emails).await;
                }
                Err(err) => {
                    eprintln!("Failed to fetch emails: {err:?}");
                }
            }
        });
        let email_arc = Arc::clone(&self.viewed_email);
        let status = Arc::clone(&self.fetching_emails);
        tokio::spawn(async move {
            while let Some(email) = rx.recv().await {
                *email_arc.lock().unwrap() = Some(email);
                *status.lock().unwrap() = false;
            }
        });
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
                subject,
                received_at,
                ..
            } in emails.clone()
            {
                let date = get_date(received_at);
                Frame::new().stroke(Stroke::default()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(&subject);
                            if ui.small_button("🗑").clicked() {
                                self.delete(&id);
                            }
                            if ui.small_button("📩").clicked() {
                                self.get_email(&id);
                            }
                        });
                        ui.small(from_address);
                        ui.small(date);
                    })
                });
                ui.separator();
            }
            if *self.email_count.lock().unwrap() as usize > emails.len()
                && ui.button("Load more").clicked()
            {
                self.fetch_more_emails();
            };
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

    fn fetch_more_emails(&mut self) {
        self.fetch_emails(true);
    }
}

fn get_date(received_at: i64) -> String {
    let date = chrono::Local.timestamp_opt(received_at, 0).unwrap();
    let (pm, hour) = date.hour12();
    let minute = date.minute();
    let weekday = date.weekday().to_string();
    let month = Month::try_from(date.month() as u8).unwrap().name();
    let day = date.day();
    let year = date.year();

    let date = format!(
        "{weekday}, {month} {day}, {year} at {hour}:{minute} {}",
        if pm { "PM" } else { "AM" }
    );
    date
}

impl App for TmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);

        self.top_bar(ctx);

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.set_width(ui.available_width());

                ui.heading(self.email());
                self.email_edit_view(ui);

                let viewed_email = self.viewed_email.clone();
                let mut viewed_email = viewed_email.lock().unwrap();

                if viewed_email.is_some() && ui.button("Back to email list").clicked() {
                    *viewed_email = None;
                } else if viewed_email.is_none()
                    && !self.name.is_empty()
                    && ui.button("Fetch emails").clicked()
                {
                    self.fetch_emails(false);
                    self.count_emails();
                }

                ui.spacing();

                if *self.fetching_emails.lock().unwrap() {
                    ui.centered_and_justified(|ui| Spinner::new().size(50.).ui(ui));
                } else if let Some(email) = viewed_email.clone() {
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
        let date = get_date(received_at);
        ui.vertical(|ui| {
            ui.label(RichText::new(subject).size(40.).strong());
            Frame::group(ui.style())
                .inner_margin(Margin::symmetric(16, 16))
                .corner_radius(ui.style().visuals.menu_corner_radius)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(format!("{:10} {from_address}", "From"));
                    ui.label(format!("{:10} {to_address}", "To"));
                    ui.spacing();
                    ui.small(date);
                    Separator::default().spacing(20.).ui(ui);
                    ui.horizontal_wrapped(|ui| ui.label(text_content.unwrap_or_default()));
                });
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
