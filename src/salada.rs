extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;
extern crate time;

#[macro_use]
extern crate log;

mod logger;
mod db;
mod util;
mod record;

use std::default::Default;
use std::io::Write;
use std::io::Read;

use std::fs::File;

use hyper::server::{Request, Response};
use hyper::method::Method::{Post, Get};
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use rustc_serialize::json::{Json,ToJson};

use jmap::parse::FromJson;
use jmap::method::{RequestBatch, ResponseBatch, ResponseMethod};
use jmap::method::RequestMethod::*;
use jmap::method::ResponseMethod::*;

use util::RequestContext;
use record::RecordHandler;
use db::Db;


macro_rules! make_crud_method_dispatcher {
    ($method: expr, $rmethods: expr, $r: expr,
     $($ty: ty { $get: ident => $rget: ident, $set: ident => $rset: ident, $getup: ident => $rgetup: ident }),*) => {
        $rmethods = match $method {
            $(
                $get(args, id)
                    => vec!($r.get_records(&args).map(|a| $rget(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
                $set(args, id)
                    => vec!($r.set_records(&args).map(|a| $rset(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
                $getup(args, id) => $r.get_record_updates(&args).map(|a| {
                    match a {
                        (a, Some(b)) => vec!($rgetup(a, id.clone()), $rget(b, id.clone())),
                        (a, _)       => vec!($rgetup(a, id.clone())),
                    }
                }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),
            )*
            RequestError(args, id) => vec!(ResponseError(args, id)),
        };
    }
}


fn jmap_handler(batch: RequestBatch) -> ResponseBatch {
    let mut rbatch: ResponseBatch = ResponseBatch::default();

    let r = RequestContext {
        userid: 1, // XXX get userid from auth
        db: Db::open().unwrap(),
    };

    for method in batch.0.into_iter() {
        let rmethods: Vec<ResponseMethod>;
        make_crud_method_dispatcher!(method, rmethods, r,
            Calendar {
                GetCalendars       => Calendars,
                SetCalendars       => CalendarsSet,
                GetCalendarUpdates => CalendarUpdates
            },
            CalendarEvent {
                GetCalendarEvents       => CalendarEvents,
                SetCalendarEvents       => CalendarEventsSet,
                GetCalendarEventUpdates => CalendarEventUpdates
            },
            Contact {
                GetContacts       => Contacts,
                SetContacts       => ContactsSet,
                GetContactUpdates => ContactUpdates
            },
            ContactGroup {
                GetContactGroups       => ContactGroups,
                SetContactGroups       => ContactGroupsSet,
                GetContactGroupUpdates => ContactGroupUpdates
            },
            Mailbox {
                GetMailboxes      => Mailboxes,
                SetMailboxes      => MailboxesSet,
                GetMailboxUpdates => MailboxUpdates
            }
        );

        rbatch.0.extend(rmethods.into_iter());
    }

    rbatch
}

fn file_handler(path: &String) -> String {
    let mut f = File::open(String::from("client")+path).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
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
            Err(e) => error!("response error: {}", e),
            _      => (),
        };
}

fn http_handler(mut req: Request, mut res: Response) {
    res.headers_mut().set(header::Server("salada/0.0.5".to_string()));

    let method = req.method.clone();
    let uri = req.uri.clone();

    match (method, uri) {
        (Post, AbsolutePath(ref path)) if path == "/jmap" => {
            match Json::from_reader(&mut req) {
                Ok(j) => match RequestBatch::from_json(&j) {
                    Ok(b) =>
                        finish_response(res, StatusCode::Ok, Some(jmap_handler(b).to_json().to_string().as_bytes())),
                    Err(e) =>
                        finish_response(res, StatusCode::BadRequest, Some(e.to_string().into_bytes().as_ref())),
                },
                Err(e) =>
                    finish_response(res, StatusCode::BadRequest, Some(e.to_string().into_bytes().as_ref())),
            }
        },

        (_, AbsolutePath(ref path)) if path == "/jmap" => finish_response(res, StatusCode::MethodNotAllowed, None),

        (Get, AbsolutePath(ref path)) => {
            finish_response(res, StatusCode::Ok, Some(file_handler(path).as_ref()))
        },

        _ => finish_response(res, StatusCode::NotFound, None),
    };
}

fn main() {
    logger::init().unwrap();

    info!("Listening on http://127.0.0.1:3000/jmap");
    hyper::Server::http("127.0.0.1:3000").unwrap().handle(http_handler).unwrap();
}
