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
mod calendar;
mod calendarevent;
mod contact;
mod contactgroup;
mod mailbox;

use std::default::Default;
use std::io::Write;

use hyper::server::{Request, Response};
use hyper::method::Method::Post;
use hyper::status::StatusCode;
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header;

use rustc_serialize::json::{Json,ToJson};

use jmap::parse::FromJson;
use jmap::method::{RequestBatch, ResponseBatch, ResponseMethod};
use jmap::method::RequestMethod::*;
use jmap::method::ResponseMethod::*;

use util::RequestContext;
use calendar::CalendarHandler;
use calendarevent::CalendarEventHandler;
use contact::ContactHandler;
use contactgroup::ContactGroupHandler;
use mailbox::MailboxHandler;
use db::Db;


fn jmap_handler(batch: RequestBatch) -> ResponseBatch {
    let mut rbatch: ResponseBatch = ResponseBatch::default();

    let r = RequestContext {
        userid: 1, // XXX get userid from auth
        db: Db::open().unwrap(),
    };

    for method in batch.0.into_iter() {
        let rmethods: Vec<ResponseMethod> = match method {
            GetCalendars(args, id)
                => vec!(r.get_calendars(&args).map(|a| Calendars(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            SetCalendars(args, id)
                => vec!(r.set_calendars(&args).map(|a| CalendarsSet(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            GetCalendarUpdates(args, id) => r.get_calendar_updates(&args).map(|a| {
                match a {
                    (a, Some(b)) => vec!(CalendarUpdates(a, id.clone()), Calendars(b, id.clone())),
                    (a, _)       => vec!(CalendarUpdates(a, id.clone())),
                }
            }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),

            GetCalendarEvents(args, id)
                => vec!(r.get_calendar_events(&args).map(|a| CalendarEvents(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            SetCalendarEvents(args, id)
                => vec!(r.set_calendar_events(&args).map(|a| CalendarEventsSet(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            GetCalendarEventUpdates(args, id) => r.get_calendar_event_updates(&args).map(|a| {
                match a {
                    (a, Some(b)) => vec!(CalendarEventUpdates(a, id.clone()), CalendarEvents(b, id.clone())),
                    (a, _)       => vec!(CalendarEventUpdates(a, id.clone())),
                }
            }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),

            GetContacts(args, id)
                => vec!(r.get_contacts(&args).map(|a| Contacts(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            SetContacts(args, id)
                => vec!(r.set_contacts(&args).map(|a| ContactsSet(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            GetContactUpdates(args, id) => r.get_contact_updates(&args).map(|a| {
                match a {
                    (a, Some(b)) => vec!(ContactUpdates(a, id.clone()), Contacts(b, id.clone())),
                    (a, _)       => vec!(ContactUpdates(a, id.clone())),
                }
            }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),

            GetContactGroups(args, id)
                => vec!(r.get_contactgroups(&args).map(|a| ContactGroups(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            SetContactGroups(args, id)
                => vec!(r.set_contactgroups(&args).map(|a| ContactGroupsSet(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            GetContactGroupUpdates(args, id) => r.get_contactgroup_updates(&args).map(|a| {
                match a {
                    (a, Some(b)) => vec!(ContactGroupUpdates(a, id.clone()), ContactGroups(b, id.clone())),
                    (a, _)       => vec!(ContactGroupUpdates(a, id.clone())),
                }
            }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),

            GetMailboxes(args, id)
                => vec!(r.get_mailboxes(&args).map(|a| Mailboxes(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            SetMailboxes(args, id)
                => vec!(r.set_mailboxes(&args).map(|a| MailboxesSet(a, id.clone())).unwrap_or_else(|e| ResponseError(e, id.clone()))),
            GetMailboxUpdates(args, id) => r.get_mailbox_updates(&args).map(|a| {
                match a {
                    (a, Some(b)) => vec!(MailboxUpdates(a, id.clone()), Mailboxes(b, id.clone())),
                    (a, _)       => vec!(MailboxUpdates(a, id.clone())),
                }
            }).unwrap_or_else(|e| vec!(ResponseError(e, id.clone()))),

            RequestError(args, id) => vec!(ResponseError(args, id)),
        };

        rbatch.0.extend(rmethods.into_iter());
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
            Err(e) => error!("response error: {}", e),
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
                        Ok(b) =>
                            finish_response(res, StatusCode::Ok, Some(jmap_handler(b).to_json().to_string().as_bytes())),
                        Err(e) =>
                            finish_response(res, StatusCode::BadRequest, Some(e.to_string().into_bytes().as_ref())),
                    },
                    Err(e) =>
                        finish_response(res, StatusCode::BadRequest, Some(e.to_string().into_bytes().as_ref())),
                }
            },

            (_, "/jmap") => finish_response(res, StatusCode::MethodNotAllowed, None),
            _            => finish_response(res, StatusCode::NotFound, None),
        },
        _ => finish_response(res, StatusCode::BadRequest, None),
    };
}

fn main() {
    logger::init().unwrap();

    let server = hyper::Server::http(http_handler);
    let _listen_guard = server.listen("127.0.0.1:3000").unwrap();
    info!("Listening on http://127.0.0.1:3000/jmap");
}
