extern crate hyper;
extern crate rustc_serialize;
extern crate rusqlite;
extern crate jmap;
extern crate time;
extern crate mime_guess;

#[macro_use]
extern crate log;

mod logger;
mod db;
mod util;
mod record;

use std::default::Default;
use std::io::{Read, Write, ErrorKind};

use std::fs::File;
use std::path::Path;

use hyper::server::{Request, Response};
use hyper::method::Method;
use hyper::method::Method::{Post, Get, Head};
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


struct StatusBody {
    code: StatusCode,
    body: Option<Vec<u8>>
}
impl StatusBody {
    fn new(code: StatusCode, body: Option<Vec<u8>>) -> StatusBody {
        StatusBody { code: code, body: body }
    }
}


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


fn jmap_handler(mut req: Request) -> StatusBody {

    match Json::from_reader(&mut req) {
        Ok(j) => match RequestBatch::from_json(&j) {
            Ok(b) => {
                let mut rbatch: ResponseBatch = ResponseBatch::default();

                let r = RequestContext {
                    userid: 1, // XXX get userid from auth
                    db: Db::open().unwrap(),
                };

                for method in b.0.into_iter() {
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

                StatusBody::new(StatusCode::Ok, Some(rbatch.to_json().to_string().into_bytes()))
            },
            Err(e) =>
                StatusBody::new(StatusCode::BadRequest, Some(e.to_string().into_bytes())),
        },
        Err(e) => StatusBody::new(StatusCode::BadRequest, Some(e.to_string().into_bytes())),
    }
}

fn file_handler(path: &String, res: &mut Response, include_body: bool) -> StatusBody {
    let mut pathonly: String = (*path).clone();
    if let Some(n) = pathonly.find(|c| c == '?' || c == '#') {
        pathonly.truncate(n);
    }

    let fullpath = String::from("client") + if pathonly == "/" { "/index.html" } else { &pathonly };
    let mimetype = mime_guess::guess_mime_type(&Path::new(&fullpath));

    let sb = match File::open(fullpath) {
        Err(ref e) => match e.kind() {
            ErrorKind::NotFound => StatusBody::new(StatusCode::NotFound, None),
            // XXX others
            _ => StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
        },
        Ok(ref mut f) => {
            match include_body {
                false => StatusBody::new(StatusCode::Ok, None),
                true  => {
                    let mut buf: Vec<u8> = vec!();
                    match f.read_to_end(&mut buf) {
                        Err(ref e) =>
                            StatusBody::new(StatusCode::InternalServerError, Some(format!("{}", e).into_bytes())),
                        _ =>
                            StatusBody::new(StatusCode::Ok, Some(buf)),
                    }
                },
            }
        },
    };

    let headers = res.headers_mut();
    headers.set(header::ContentType(mimetype));
    if let Some(ref b) = sb.body {
        headers.set(header::ContentLength((*b).len() as u64));
    }

    sb
}

fn finish_response(method: Method, path: &String, mut res: Response, out: StatusBody) {
    *res.status_mut() = out.code;

    info!("HTTP: {} {} => {}", method, path, out.code);

    match res.start()
        .and_then(|mut res|
                  match out.body {
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
        (Post, AbsolutePath(ref path)) if path == "/jmap/" => {
            let sb = jmap_handler(req);
            finish_response(Post, path, res, sb)
        },

        (Get, AbsolutePath(ref path)) => {
            let sb = file_handler(path, &mut res, true);
            finish_response(Get, path, res, sb)
        },

        (Head, AbsolutePath(ref path)) => {
            let sb = file_handler(path, &mut res, false);
            finish_response(Head, path, res, sb)
        },

        (_, ref ap) => {
            let s = match ap {
                &AbsolutePath(ref u) => u,
                _ => panic!("RequestUri not AbsolutePath?"),
            };
            let mut drain: Vec<u8> = vec!();
            req.read_to_end(&mut drain).ok();
            finish_response(req.method, &s, res, StatusBody::new(StatusCode::MethodNotAllowed, None))
        },
    };
}

fn main() {
    logger::init().unwrap();

    info!("Listening on http://127.0.0.1:3000/jmap");
    hyper::Server::http("127.0.0.1:3000").unwrap().handle(http_handler).unwrap();
}
