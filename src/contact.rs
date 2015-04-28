use std::default::Default;
use jmap::method::*;
use jmap::method::ResponseMethod::*;

use util::RequestContext;

pub trait ContactHandler {
    fn get_contacts(&self, args: GetRequestArgs, client_id: String) -> ResponseMethod;
    fn get_contact_updates(&self, args: GetUpdatesRequestArgs, client_id: String) -> ResponseMethod;
    fn set_contacts(&self, args: SetRequestArgs, client_id: String) -> ResponseMethod;
}

impl ContactHandler for RequestContext {
    fn get_contacts(&self, args: GetRequestArgs, client_id: String) -> ResponseMethod {
        println!("get_contacts: {:?} {}", args, client_id);

        let records = self.db.get_records(args.ids.as_option());

        let response = GetResponseArgs {
            state: "abc123".to_string(),
            list: Some(records.unwrap().iter().map(|ref r| r.to_partial()).collect()),
            not_found: None,
        };

        println!("{:?}", response);

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
