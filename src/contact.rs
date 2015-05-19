use jmap::method::*;
use jmap::contact::Contact;

use util::RequestContext;
use record::RecordHandler;

pub trait ContactHandler {
    fn get_contacts(&self, args: &GetRequestArgs<Contact>)               -> Result<GetResponseArgs<Contact>,MethodError>;
    fn get_contact_updates(&self, args: &GetUpdatesRequestArgs<Contact>) -> Result<GetUpdatesResponseArgs<Contact>,MethodError>;
    fn set_contacts(&self, args: &SetRequestArgs<Contact>)               -> Result<SetResponseArgs<Contact>,MethodError>;
}

impl ContactHandler for RequestContext {
    fn get_contacts(&self, args: &GetRequestArgs<Contact>) -> Result<GetResponseArgs<Contact>,MethodError> {
        self.get_records(args)
    }

    fn get_contact_updates(&self, args: &GetUpdatesRequestArgs<Contact>) -> Result<GetUpdatesResponseArgs<Contact>,MethodError> {
        self.get_record_updates(args)
    }

    fn set_contacts(&self, args: &SetRequestArgs<Contact>) -> Result<SetResponseArgs<Contact>,MethodError> {
        self.set_records(args)
    }
}
