use jmap::method::*;

pub fn get_contacts(args: GetRequestArgs, client_id: String) {
    println!("get_contacts: {:?} {}", args, client_id);
}

pub fn get_contact_updates(args: GetUpdatesRequestArgs, client_id: String) {
    println!("get_contact_updates: {:?} {}", args, client_id);
}

pub fn set_contacts(args: SetRequestArgs, client_id: String) {
    println!("set_contacts: {:?} {}", args, client_id);
}
