use hyper::server::Request;
use hyper::status::StatusCode;

use rustc_serialize::json::{Json,ToJson};

use jmap::parse::FromJson;
use jmap::method::{RequestBatch, ResponseBatch, ResponseMethod};
use jmap::method::RequestMethod::*;
use jmap::method::ResponseMethod::*;

use http_handler::StatusBody;

use record::RecordHandler;

use util::RequestContext;
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


pub fn handler(mut req: Request) -> StatusBody {

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
                        },
                        Message {
                            GetMessages       => Messages,
                            SetMessages       => MessagesSet,
                            GetMessageUpdates => MessageUpdates
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
