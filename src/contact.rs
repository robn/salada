use std::default::Default;
use jmap::method::*;
use jmap::method::ResponseMethod::*;

use db::Db;

pub fn get_contacts(args: GetRequestArgs, client_id: String) -> ResponseMethod {
    println!("get_contacts: {:?} {}", args, client_id);

    let db = Db::open().unwrap();

    let records = db.get_records(args.ids.as_option());

    let response = GetResponseArgs {
        state: "abc123".to_string(),
        list: Some(records.unwrap().iter().map(|ref r| r.to_partial()).collect()),
        not_found: None,
    };

    println!("{:?}", response);

    Contacts(response, client_id)
}

pub fn get_contact_updates(args: GetUpdatesRequestArgs, client_id: String) -> ResponseMethod {
    println!("get_contact_updates: {:?} {}", args, client_id);
    ContactUpdates(GetUpdatesResponseArgs::default(), client_id)
}

pub fn set_contacts(args: SetRequestArgs, client_id: String) -> ResponseMethod {
    println!("set_contacts: {:?} {}", args, client_id);
    ContactsSet(SetResponseArgs::default(), client_id)
}
