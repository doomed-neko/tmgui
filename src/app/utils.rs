use chrono::{Datelike, Month, TimeZone, Timelike};
use rand::Rng;

use crate::{
    app::TempMailApp,
    event_handler::{Event, EventResponse},
};
impl TempMailApp {
    pub(super) fn gen_random_name(len: usize) -> String {
        rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(len)
            .map(|x| x as char)
            .collect::<String>()
    }
    pub(super) fn get_date(received_at: i64) -> String {
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
    pub(super) fn email(&self) -> String {
        [self.name.clone(), self.domain.clone()].join("@")
    }

    pub(super) fn send_event(&mut self, event: Event) {
        self.fetching = true;
        let _ = self.events.send(event);
    }

    pub(super) fn handle_responses(&mut self) {
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
}
