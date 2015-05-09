use std::default::Default;
use std::collections::HashSet;
use jmap::method::*;
use jmap::util::Presence::*;

use util::RequestContext;

pub trait ContactHandler {
    fn get_contacts(&self, args: &GetRequestArgs)               -> Result<GetResponseArgs,MethodError>;
    fn get_contact_updates(&self, args: &GetUpdatesRequestArgs) -> Result<GetUpdatesResponseArgs,MethodError>;
    fn set_contacts(&self, args: &SetRequestArgs)               -> Result<SetResponseArgs,MethodError>;
}

impl ContactHandler for RequestContext {
    fn get_contacts(&self, args: &GetRequestArgs) -> Result<GetResponseArgs,MethodError> {
        let (records, state) = try!(self.db.transaction(|| {
            Ok((
                try!(self.db.get_records(self.userid, args.ids.as_option(), args.since_state.as_option())),
                try!(self.db.get_state(self.userid)),
            ))
        }));

        let not_found = match args.ids {
            Absent => None,
            Present(ref ids) => {
                let mut found = HashSet::new();
                for record in records.iter() {
                    found.insert(&record.id);
                }
                let not_found = ids.into_iter().filter(|id| !found.contains(id)).map(|id| id.clone()).collect::<Vec<_>>();
                match not_found.len() {
                    0 => None,
                    _ => Some(not_found),
                }
            }
        };

        let list = match args.properties {
            Absent         => Some(records.iter().map(|ref r| r.to_partial()).collect()),
            Present(ref p) => Some(records.iter().map(|ref r| r.to_filtered_partial(p)).collect()),
        };

        let response = GetResponseArgs {
            state: state,
            list: list,
            not_found: not_found,
        };

        Ok(response)
    }

    fn get_contact_updates(&self, args: &GetUpdatesRequestArgs) -> Result<GetUpdatesResponseArgs,MethodError> {
        println!("get_contact_updates: {:?}", args);
        Ok(GetUpdatesResponseArgs::default())
    }

    fn set_contacts(&self, args: &SetRequestArgs) -> Result<SetResponseArgs,MethodError> {
        println!("set_contacts: {:?}", args);
        Ok(SetResponseArgs::default())
    }
}
