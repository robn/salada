use jmap::method::*;

use db::Db;

pub fn get_contacts(args: GetRequestArgs, client_id: String) {
    println!("get_contacts: {:?} {}", args, client_id);

    let db = Db::open().unwrap();

    let records = db.get_records(args.ids.as_option());

    println!("{:?}", records);
}

pub fn get_contact_updates(args: GetUpdatesRequestArgs, client_id: String) {
    println!("get_contact_updates: {:?} {}", args, client_id);
}

pub fn set_contacts(args: SetRequestArgs, client_id: String) {
    println!("set_contacts: {:?} {}", args, client_id);
}
