use std::sync::mpsc::{Receiver, Sender};

use tmapi::{Client, Email};
use tokio::runtime::Handle;

pub enum Event {
    DeleteAllEmails(String),
    DeleteEmail(String, usize),
    FetchEmails(String),
    FetchMoreEmails(String, u32),
    FetchEmail(String),
    FetchDomanins,
    CountEmails(String),
}

pub enum EventResponse {
    Domains(Vec<String>),
    Emails(Vec<Email>),
    EmailsMore(Vec<Email>),
    Email(Email),
    Count(u32),
    EmailsDeleted,
    EmailDeleted(usize),
}

pub struct Handler {
    event_stream: Receiver<Event>,
    response_stream: Sender<EventResponse>,
}

impl Handler {
    pub fn new(event_stream: Receiver<Event>, response_stream: Sender<EventResponse>) -> Self {
        Self {
            event_stream,
            response_stream,
        }
    }
    pub fn listen(&self) {
        while let Ok(event) = self.event_stream.recv() {
            match event {
                Event::DeleteAllEmails(email) => self.delete_all(email),
                Event::DeleteEmail(id, index) => self.delete(id, index),
                Event::FetchEmails(email) => self.fetch_emails(email, 0),
                Event::FetchEmail(id) => self.fetch_email(id),
                Event::FetchDomanins => self.fetch_domains(),
                Event::CountEmails(email) => self.fetch_count(email),
                Event::FetchMoreEmails(email, offset) => self.fetch_emails(email, offset),
            }
        }
    }

    fn delete_all(&self, email: String) {
        let client = Client::new(email).unwrap();
        let count = Handle::current().block_on(client.delete_all_emails());
        match count {
            Ok(_) => {
                let _ = self.response_stream.send(EventResponse::EmailsDeleted);
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }
    fn delete(&self, id: String, index: usize) {
        let client = Client::new("example@example.com").unwrap();
        let status = Handle::current().block_on(client.delete_inbox(id));
        match status {
            Ok(()) => {
                let _ = self
                    .response_stream
                    .send(EventResponse::EmailDeleted(index));
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }

    fn fetch_emails(&self, email: String, offset: u32) {
        let client = Client::new(email).unwrap();
        let emails = Handle::current().block_on(client.get_emails(50, 0));
        if let Ok(emails) = emails {
            if offset == 0 {
                let _ = self.response_stream.send(EventResponse::Emails(emails));
            } else {
                let _ = self.response_stream.send(EventResponse::EmailsMore(emails));
            }
        }
    }
    fn fetch_email(&self, id: String) {
        let client = Client::new("example@example.com").unwrap();
        let email = Handle::current().block_on(client.get_inbox(id));
        if let Ok(email) = email {
            let _ = self.response_stream.send(EventResponse::Email(email));
        }
    }

    fn fetch_count(&self, email: String) {
        let client = Client::new(email).unwrap();
        let count = Handle::current().block_on(client.email_count());
        if let Ok(count) = count {
            let _ = self.response_stream.send(EventResponse::Count(count));
        }
    }

    fn fetch_domains(&self) {
        let client = Client::new("example@example.com").unwrap();
        let domains = Handle::current().block_on(client.get_domains());
        match domains {
            Ok(domains) => {
                let _ = self.response_stream.send(EventResponse::Domains(domains));
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }
}
