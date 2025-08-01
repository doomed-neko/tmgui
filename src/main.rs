use crate::config::set_styles;
use std::sync::mpsc::{Receiver, Sender, channel};

use chrono::{Datelike, Month, TimeZone, Timelike};
use eframe::{
    App,
    egui::{
        self, CentralPanel, ComboBox, Frame, Margin, RichText, ScrollArea, Separator, Spinner,
        Stroke, TopBottomPanel, ViewportBuilder, Widget,
    },
};
use rand::Rng;
use tmapi::Email;
use tokio::task::spawn_blocking;

use crate::event_handler::{Event, EventResponse, Handler};

mod config;
mod event_handler;

#[tokio::main]
async fn main() -> eframe::Result {
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
                EventResponse::EmailDeleted(index) => {
                    self.emails.remove(index);
                }
            }
        };
    }
}

impl App for TempMailApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.scan_responses();
        set_styles(ctx);

        TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Actions", |ui| {
                    if !self.name.is_empty()
                        && !self.emails.is_empty()
                        && ui.button("Delete all emails").clicked()
                    {
                        self.send_event(Event::DeleteAllEmails(self.email()));
                        self.send_event(Event::FetchEmails(self.email()));
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                })
            })
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.set_width(ui.available_width());

                ui.heading(self.email());
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

                if self.viewed_email.is_some() && ui.button("Back to email list").clicked() {
                    self.viewed_email = None;
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
                    let Email {
                        from_address,
                        to_address,
                        subject,
                        received_at,
                        text_content,
                        ..
                    } = email;
                    ScrollArea::vertical().show(ui, |ui| {
                        let date = Self::get_date(received_at);
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
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(text_content.unwrap_or_default())
                                    });
                                });
                        })
                    });
                } else {
                    let _ctx = ctx;
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
                        if self.email_count as usize > self.emails.len()
                            && ui.button("Load more").clicked()
                        {
                            self.send_event(Event::FetchMoreEmails(
                                self.email(),
                                self.current_offset,
                            ));
                            self.current_offset += 1;
                        };
                    });
                };
            });
        });
    }
}
