use chrono::TimeZone;
use eframe::{
    App,
    egui::{
        self, CentralPanel, ComboBox, Context, FontId, Frame, ScrollArea, Stroke, TextStyle,
        TopBottomPanel, ViewportBuilder,
    },
};
use futures::executor::block_on;
use rand::Rng;
use tmapi::{Client, Email};

struct TmApp {
    domain: String,
    domains: Vec<String>,
    name: String,
    emails: Vec<Email>,
    viewed_email: Option<Email>,
}

impl Default for TmApp {
    fn default() -> Self {
        let client = Client::new("email@example.com").unwrap();
        let domains = futures::executor::block_on(client.get_domains()).unwrap();
        let name = gen_random_name();

        Self {
            domains,
            domain: "iusearch.lol".into(),
            name,
            emails: vec![],
            viewed_email: None,
        }
    }
}

fn gen_random_name() -> String {
    rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(10)
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
        match block_on(self.client().get_emails(50, 0)) {
            Ok(emails) => {
                println!("{emails:?}");
                self.emails = emails;
            }
            Err(e) => {
                println!("{e}")
            }
        }
    }
    fn top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Actions", |ui| {
                    if !self.name.is_empty()
                        && !self.emails.is_empty()
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
            for Email {
                id,
                from_address,
                to_address,
                subject,
                received_at,
                ..
            } in self.emails.clone()
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
}

impl App for TmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);
        self.top_bar(ctx);
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading(self.email());
                ui.horizontal(|ui| {
                    let label = ui.label("Name");
                    ui.text_edit_singleline(&mut self.name)
                        .labelled_by(label.id);
                    if ui.button("↻").clicked() {
                        self.name = gen_random_name();
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
                if !self.name.is_empty() && ui.button("Fetch emails").clicked() {
                    self.fetch_emails();
                }
                if self.viewed_email.is_some() && ui.button("Back to email list").clicked() {
                    self.viewed_email = None;
                }
                ui.spacing();
                if let Some(Email {
                    // id,
                    from_address,
                    to_address,
                    subject,
                    received_at,
                    text_content,
                    ..
                }) = self.viewed_email.clone()
                {
                    ScrollArea::vertical().show(ui, |ui| {
                        let date = chrono::Local
                            .timestamp_opt(received_at, 0)
                            .unwrap()
                            .to_string();
                        ui.vertical(|ui| {
                            ui.label(format!("subject: {subject}"));
                            ui.label(format!("from: {from_address}"));
                            ui.label(format!("to: {to_address}"));
                            ui.label(format!("time: {date}"));
                            ui.horizontal_wrapped(|ui| ui.label(text_content.unwrap_or_default()));
                        })
                    });
                } else {
                    self.email_list(ctx, ui);
                }
            });
        });
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_resizable(true)
            .with_app_id("adenosine.tmgui")
            .with_inner_size([600., 1000.]),
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
