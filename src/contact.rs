use std::default::Default;
use std::collections::HashSet;
use jmap::method::*;
use jmap::method::ResponseMethod::*;
use jmap::util::Presence::*;

use util::RequestContext;

pub trait ContactHandler {
    fn get_contacts(&self, args: GetRequestArgs, client_id: String) -> ResponseMethod;
    fn get_contact_updates(&self, args: GetUpdatesRequestArgs, client_id: String) -> ResponseMethod;
    fn set_contacts(&self, args: SetRequestArgs, client_id: String) -> ResponseMethod;
}

impl ContactHandler for RequestContext {
    fn get_contacts(&self, args: GetRequestArgs, client_id: String) -> ResponseMethod {
        // XXX assuming success through here
        let txn = self.db.transaction().unwrap();
        let records = self.db.get_records(self.userid, args.ids.as_option()).unwrap();
        let state = self.db.get_state(self.userid).unwrap();
        txn.commit().unwrap();

        let not_found = match args.ids {
            Absent => None,
            Present(ids) => {
                let mut found = HashSet::new();
                for record in records.iter() {
                    found.insert(&record.id);
                }
                let not_found = ids.into_iter().filter(|id| !found.contains(id)).collect::<Vec<_>>();
                match not_found.len() {
                    0 => None,
                    _ => Some(not_found),
                }
            }
        };

        let list = match args.properties {
            Absent     => Some(records.iter().map(|ref r| r.to_partial()).collect()),
            Present(p) => Some(records.iter().map(|ref r| r.to_filtered_partial(&p)).collect()),
        };

        let response = GetResponseArgs {
            state: state,
            list: list,
            not_found: not_found,
        };

        Contacts(response, client_id)
    }

    fn get_contact_updates(&self, args: GetUpdatesRequestArgs, client_id: String) -> ResponseMethod {
        println!("get_contact_updates: {:?} {}", args, client_id);
        ContactUpdates(GetUpdatesResponseArgs::default(), client_id)
    }

    fn set_contacts(&self, args: SetRequestArgs, client_id: String) -> ResponseMethod {
        println!("set_contacts: {:?} {}", args, client_id);
        ContactsSet(SetResponseArgs::default(), client_id)
    }
}
