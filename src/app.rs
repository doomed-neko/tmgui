use std::sync::mpsc::{Receiver, Sender};

use crate::config::set_styles;

use eframe::App;
use tmapi::{Attachment, Email};

use crate::event_handler::{Event, EventResponse};

pub(super) mod ui;
pub(super) mod utils;

pub struct TempMailApp {
    domain: String,
    domains: Vec<String>,
    name: String,
    emails: Vec<Email>,
    fetching: bool,
    viewed_email: Option<Email>,
    email_count: u32,
    current_offset: u32,
    attachments: Option<Vec<Attachment>>,
    images: Vec<(String, String)>,
    events: Sender<Event>,
    responses: Receiver<EventResponse>,
}

impl TempMailApp {
    pub fn new(
        tx: Sender<Event>,
        rx: Receiver<EventResponse>,
        name: Option<String>,
        domain: Option<String>,
    ) -> Self {
        let name = name.unwrap_or(Self::gen_random_name(10));
        let domain = domain.unwrap_or("vwh.sh".into());
        let _ = tx.send(Event::FetchEmails([name.clone(), domain.clone()].join("@")));
        let _ = tx.send(Event::FetchDomanins);

        Self {
            name,
            domain,
            events: tx,
            responses: rx,
            fetching: true,
            images: Default::default(),
            emails: Default::default(),
            viewed_email: Default::default(),
            domains: Default::default(),
            email_count: Default::default(),
            current_offset: Default::default(),
            attachments: Default::default(),
        }
    }
}

impl App for TempMailApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        set_styles(ctx);
        self.handle_responses();
        self.app_ui(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("name", self.name.clone());
        storage.set_string("domain", self.domain.clone());
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}
