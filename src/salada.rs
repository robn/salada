extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;
extern crate uuid;

mod db;
mod util;
mod contact;

use std::default::Default;
use std::io::Write;

use hyper::server::{Request, Response};
use hyper::method::Method::Post;
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use rustc_serialize::json::{Json,ToJson};

use jmap::util::FromJson;
use jmap::method::{RequestBatch, ResponseBatch, ClientId};
use jmap::method::RequestMethod::*;
use jmap::method::ResponseMethod::*;

use util::RequestContext;
use contact::ContactHandler;
use db::Db;


fn jmap_handler(batch: RequestBatch) -> ResponseBatch {
    let mut rbatch: ResponseBatch = ResponseBatch::default();

    let r = RequestContext {
        userid: 1, // XXX get userid from auth
        db: Db::open().unwrap(),
    };

    for method in batch.0.into_iter() {
        let res = match method {
            GetContacts(ref args, ref id)       => r.get_contacts(args).map(|a| Contacts(a, id.clone())),
            GetContactUpdates(ref args, ref id) => r.get_contact_updates(args).map(|a| ContactUpdates(a, id.clone())),
            SetContacts(ref args, ref id)       => r.set_contacts(args).map(|a| ContactsSet(a, id.clone())),

            RequestError(ref args, ref id) => Ok(ResponseError(args.clone(), id.clone())),
        };

        rbatch.0.push(match res {
            Ok(r)  => r,
            Err(e) => ResponseError(e, method.client_id()),
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
