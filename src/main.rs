use std::sync::mpsc::channel;

use eframe::egui::ViewportBuilder;
use tokio::task::spawn_blocking;

use crate::{
    app::TempMailApp,
    event_handler::{Event, EventResponse, Handler},
};

mod app;
mod config;
mod event_handler;
#[tokio::main]
async fn main() -> eframe::Result {
    pretty_env_logger::init();
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_app_id("adenosine.tmgui"),
        ..Default::default()
    };
    let (tx_event, rx_event) = channel::<Event>();
    let (tx_response, rx_response) = channel::<EventResponse>();
    let handler = Handler::new(rx_event, tx_response);
    spawn_blocking(move || handler.listen());

    eframe::run_native(
        "TMApi",
        opts,
        Box::new(|c| {
            let name = c.storage.and_then(|x| x.get_string("name"));
            let domain = c.storage.and_then(|x| x.get_string("domain"));
            Ok(Box::new(TempMailApp::new(
                tx_event,
                rx_response,
                name,
                domain,
            )))
        }),
    )
}
