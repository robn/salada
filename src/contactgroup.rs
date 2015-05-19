use jmap::method::*;
use jmap::contactgroup::ContactGroup;

use util::RequestContext;
use record::RecordHandler;

pub trait ContactGroupHandler {
    fn get_contactgroups(&self, args: &GetRequestArgs<ContactGroup>)               -> Result<GetResponseArgs<ContactGroup>,MethodError>;
    fn get_contactgroup_updates(&self, args: &GetUpdatesRequestArgs<ContactGroup>) -> Result<(GetUpdatesResponseArgs<ContactGroup>,Option<GetResponseArgs<ContactGroup>>),MethodError>;
    fn set_contactgroups(&self, args: &SetRequestArgs<ContactGroup>)               -> Result<SetResponseArgs<ContactGroup>,MethodError>;
}

impl ContactGroupHandler for RequestContext {
    fn get_contactgroups(&self, args: &GetRequestArgs<ContactGroup>) -> Result<GetResponseArgs<ContactGroup>,MethodError> {
        self.get_records(args)
    }

    fn get_contactgroup_updates(&self, args: &GetUpdatesRequestArgs<ContactGroup>) -> Result<(GetUpdatesResponseArgs<ContactGroup>,Option<GetResponseArgs<ContactGroup>>),MethodError> {
        self.get_record_updates(args)
    }

    fn set_contactgroups(&self, args: &SetRequestArgs<ContactGroup>) -> Result<SetResponseArgs<ContactGroup>,MethodError> {
        self.set_records(args)
    }
}

