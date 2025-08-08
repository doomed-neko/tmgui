use crate::config::set_styles;
use std::{
    fmt::{Display, Write},
    sync::mpsc::{Receiver, Sender, channel},
};

use chrono::{Datelike, Month, TimeZone, Timelike};
use eframe::{
    App,
    egui::{
        self, CentralPanel, ComboBox, Frame, Grid, Margin, RichText, ScrollArea, Separator,
        Spinner, Stroke, TopBottomPanel, ViewportBuilder, Widget,
    },
};
use rand::Rng;
use tmapi::{Attachment, Email};
use tokio::task::spawn_blocking;

use crate::event_handler::{Event, EventResponse, Handler};

mod config;
mod event_handler;

struct UnitSize(u64);
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

#[tokio::main]
async fn main() -> eframe::Result {
    pretty_env_logger::init();
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_app_id("adenosine.tmgui"),
        ..Default::default()
    };
    let (tx_event, rx_event) = channel::<Event>();
    let (tx_response, rx_response) = channel::<EventResponse>();
    spawn_blocking(|| {
        let handler = Handler::new(rx_event, tx_response);
        handler.listen();
    });

    eframe::run_native(
        "TMApi",
        opts,
        Box::new(|_| Ok(Box::new(TempMailApp::new(tx_event, rx_response)))),
    )
}

struct TempMailApp {
    domain: String,
    domains: Vec<String>,
    name: String,
    emails: Vec<Email>,
    fetching: bool,
    viewed_email: Option<Email>,
    email_count: u32,
    current_offset: u32,
    attachments: Option<Vec<Attachment>>,
    events: Sender<Event>,
    responses: Receiver<EventResponse>,
}

impl TempMailApp {
    pub fn new(tx: Sender<Event>, rx: Receiver<EventResponse>) -> Self {
        let name = Self::gen_random_name(10);
        let _ = tx.send(Event::FetchDomanins);

        Self {
            name,
            fetching: true,
            events: tx,
            responses: rx,
            domain: "vwh.sh".into(),
            emails: Default::default(),
            viewed_email: Default::default(),
            domains: Default::default(),
            email_count: Default::default(),
            current_offset: Default::default(),
            attachments: Default::default(),
        }
    }
    fn gen_random_name(len: usize) -> String {
        rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(len)
            .map(|x| x as char)
            .collect::<String>()
    }
    fn get_date(received_at: i64) -> String {
        let date = chrono::Local.timestamp_opt(received_at, 0).unwrap();
        let (pm, hour) = date.hour12();
        let minute = date.minute();
        let weekday = date.weekday().to_string();
        let month = Month::try_from(date.month() as u8).unwrap().name();
        let day = date.day();
        let year = date.year();

        format!(
            "{weekday}, {month} {day}, {year} at {hour}:{minute} {}",
            if pm { "PM" } else { "AM" }
        )
    }
    pub fn email(&self) -> String {
        [self.name.clone(), self.domain.clone()].join("@")
    }

    pub fn send_event(&mut self, event: Event) {
        self.fetching = true;
        let _ = self.events.send(event);
    }

    fn scan_responses(&mut self) {
        if let Ok(response) = self.responses.try_recv() {
            self.fetching = false;
            match response {
                EventResponse::Domains(domains) => self.domains = domains,
                EventResponse::Emails(emails) => self.emails = emails,
                EventResponse::Email(email) => self.viewed_email = Some(email),
                EventResponse::Count(c) => self.email_count = c,
                EventResponse::EmailsMore(emails) => self.emails.extend(emails),
                EventResponse::EmailsDeleted => self.emails.clear(),
                EventResponse::Attachments(attachments) => self.attachments = Some(attachments),
                EventResponse::EmailDeleted(index) => {
                    self.emails.remove(index);
                }
            }
        };
    }

    fn email_tile(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        id: String,
        from_address: String,
        subject: String,
        received_at: i64,
    ) {
        let date = Self::get_date(received_at);
        Frame::new().stroke(Stroke::default()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(&subject);
                    if ui.small_button("🗑").clicked() {
                        self.send_event(Event::DeleteEmail(id.clone(), index));
                    }
                    if ui.small_button("📩").clicked() {
                        self.send_event(Event::FetchEmail(id.clone()));
                    }
                });
                ui.small(from_address);
                ui.small(date);
            })
        });
        ui.separator();
    }

    fn email_list(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            if self.emails.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.heading("No emails are here yet");
                });
                return;
            }
            for (
                index,
                Email {
                    id,
                    from_address,
                    subject,
                    received_at,
                    ..
                },
            ) in self.emails.clone().into_iter().enumerate()
            {
                self.email_tile(ui, index, id, from_address, subject, received_at);
            }
            if self.email_count as usize > self.emails.len() && ui.button("Load more").clicked() {
                self.send_event(Event::FetchMoreEmails(self.email(), self.current_offset));
                self.current_offset += 1;
            };
        });
    }

    fn email_view(&mut self, ui: &mut egui::Ui, email: Email) {
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
                        ui.label(format!("{:10} {from_address}", "From"));
                        ui.label(format!("{:10} {to_address}", "To"));
                        ui.spacing();
                        ui.small(date);
                        Separator::default().spacing(20.).ui(ui);
                        ui.horizontal_wrapped(|ui| ui.label(text_content.unwrap_or_default()));
                        if let Some(attachments) = &self.attachments {
                            attachment_list(ui, attachments);
                        } else if has_attachments
                            && self.attachments.is_none()
                            && ui
                                .button(format!(
                                    "Fetch {attachment_count} attachment{s}",
                                    s = if attachment_count == 1 { "" } else { "s" }
                                ))
                                .clicked()
                        {
                            self.send_event(Event::GetAttachments(id));
                        }
                    });
            })
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

fn attachment_list(ui: &mut egui::Ui, attachments: &[Attachment]) {
    Grid::new("attachments").show(ui, |ui| {
        for attachments in attachments.chunks(3) {
            for attachment in attachments {
                attachment_tile(ui, attachment);
            }
            ui.end_row();
        }
    });
}

fn attachment_tile(ui: &mut egui::Ui, attachment: &Attachment) {
    Frame::group(ui.style())
        .inner_margin(Margin::symmetric(5, 5))
        .corner_radius(ui.style().visuals.menu_corner_radius)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("🖼 {}", attachment.filename));
                ui.separator();
                ui.label(UnitSize(attachment.size).to_string());
                ui.separator();
                if ui.button("↓").clicked() {
                    let api_url = "https://api.barid.site";
                    open::that(format!("{api_url}/attachments/{}", attachment.id)).ok();
                }
            })
        });
}

impl App for TempMailApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.scan_responses();
        set_styles(ctx);

        TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
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
                // ui.menu_button("Actions", |ui| {})
            })
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.heading(self.email());
                    if ui.button("copy").clicked() {
                        ctx.copy_text(self.email());
                    }
                });
                self.email_selector(ui);

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
