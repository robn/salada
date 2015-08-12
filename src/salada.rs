extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;
extern crate time;
extern crate mime_guess;
extern crate uuid;

#[macro_use]
extern crate log;

mod logger;
mod db;
mod http_handler;
mod jmap_handler;
mod upload_handler;
mod static_handler;
mod util;
mod record;

fn main() {
    logger::init().unwrap();

    info!("Listening on http://127.0.0.1:3000/jmap");
    hyper::Server::http("127.0.0.1:3000").unwrap().handle(http_handler::handler).unwrap();
}
