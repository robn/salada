extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;

use std::default::Default;
use std::io::Write;

use hyper::server::{Request, Response};
use hyper::method::Method::Post;
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use rustc_serialize::json::{Json,ToJson};

use jmap::util::FromJson;
use jmap::method::{RequestBatch, ResponseBatch};
use jmap::method::RequestMethod::*;

mod db;
mod contact;

fn jmap_handler(batch: RequestBatch) -> ResponseBatch {
    let mut rbatch: ResponseBatch = ResponseBatch::default();

    for method in batch.0.into_iter() {
        rbatch.0.push(match method {
            GetContacts(args, client_id) =>
                contact::get_contacts(args, client_id),
            GetContactUpdates(args, client_id) =>
                contact::get_contact_updates(args, client_id),
            SetContacts(args, client_id) =>
                contact::set_contacts(args, client_id),
        });
    }

    rbatch
}

fn finish_response(mut res: Response, code: StatusCode, body: Option<&[u8]>) {
    *res.status_mut() = code;

    match res.start()
        .and_then(|mut res|
                  match body {
                      Some(ref b) => {
                          try!(res.write_all(b));
                          Ok(res)
                      }
                      None => {
                          Ok(res)
                      }
                  })
        .and_then(|res| res.end()) {
            Err(e) => println!("response error: {}", e),
            _      => (),
        };
}

fn http_handler(mut req: Request, mut res: Response) {
    res.headers_mut().set(header::Server("salada/0.0.1".to_string()));

    let uri = req.uri.clone();

    match uri {
        AbsolutePath(ref path) => match (&req.method, &path[..]) {

            (&Post, "/jmap") => {
                match Json::from_reader(&mut req) {
                    Ok(j) => match RequestBatch::from_json(&j) {
                        Ok(b) => {
                            return finish_response(res, StatusCode::Ok, Some(jmap_handler(b).to_json().to_string().as_bytes()))
                        },
                        Err(e) => {
                            println!("jmap parse error: {}", e);
                            return finish_response(res, StatusCode::BadRequest, None)
                        },
                    },
                    Err(e) => {
                        println!("json parse error: {}", e);
                        return finish_response(res, StatusCode::BadRequest, None)
                    },
                }
            },

            (_, "/jmap") => return finish_response(res, StatusCode::MethodNotAllowed, None),
            _            => return finish_response(res, StatusCode::NotFound, None),
        },
        _ => return finish_response(res, StatusCode::BadRequest, None)
    };
}

fn main() {
    let server = hyper::Server::http(http_handler);
    let _listen_guard = server.listen("127.0.0.1:3000").unwrap();
    println!("Listening on http://127.0.0.1:3000/jmap");
}
