extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;

use hyper::server::{Request, Response};
use hyper::method::Method::Post;
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use rustc_serialize::json::Json;
use jmap::util::FromJson;
use jmap::method::RequestBatch;
use jmap::method::RequestMethod::*;

mod db;
mod contact;

fn jmap_handler(batch: RequestBatch) {
    for method in batch.0.into_iter() {
        match method {
            GetContacts(args, client_id) =>
                contact::get_contacts(args, client_id),
            GetContactUpdates(args, client_id) =>
                contact::get_contact_updates(args, client_id),
            SetContacts(args, client_id) =>
                contact::set_contacts(args, client_id),
        }
    }
}

fn http_handler(mut req: Request, mut res: Response) {
    res.headers_mut().set(header::Server("salada/0.0.1".to_string()));

    let uri = req.uri.clone();

    *res.status_mut() = match uri {
        AbsolutePath(ref path) => match (&req.method, &path[..]) {

            (&Post, "/jmap") => {
                match Json::from_reader(&mut req) {
                    Ok(j) => match RequestBatch::from_json(&j) {
                        Ok(b) => {
                            jmap_handler(b);
                            StatusCode::Ok
                        },
                        Err(e) => {
                            println!("jmap parse error: {}", e);
                            StatusCode::BadRequest
                        },
                    },
                    Err(e) => {
                        println!("json parse error: {}", e);
                        StatusCode::BadRequest
                    },
                }
            },

            (_, "/jmap") => StatusCode::MethodNotAllowed,
            _            => StatusCode::NotFound,
        },
        _ => StatusCode::BadRequest,
    };

    match res.start().and_then(|res| res.end()) {
        Err(e) => println!("response error: {}", e),
        _      => (),
    }
}

fn main() {
    let server = hyper::Server::http(http_handler);
    let _listen_guard = server.listen("127.0.0.1:3000").unwrap();
    println!("Listening on http://127.0.0.1:3000/jmap");
}
