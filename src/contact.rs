use std::default::Default;
use std::collections::{HashSet, BTreeMap};
use jmap::method::*;
use jmap::parse::Presence::*;

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
        let (changed, removed, state) = try!(self.db.transaction(|| {
            let max_changes = match args.max_changes.as_option() {
                Some(i) => Some(*i as i64),
                None    => None,
            };
            let (changed, removed) = try!(self.db.get_record_updates(self.userid, &args.since_state, max_changes));
            Ok((
                changed,
                removed,
                try!(self.db.get_state(self.userid)),
            ))
        }));

        let response = GetUpdatesResponseArgs {
            old_state: args.since_state.clone(),
            new_state: state,
            changed:   changed,
            removed:   removed,
        };

        Ok(response)
    }

    fn set_contacts(&self, args: &SetRequestArgs) -> Result<SetResponseArgs,MethodError> {
        let res = try!(self.db.exclusive(|| {
            if let Present(ref s) = args.if_in_state {
                try!(self.db.check_state(self.userid, s));
            }

            let old_state = try!(self.db.get_state(self.userid));

            let create = match args.create {
                Present(ref c) if c.len() > 0 => Some(c),
                _                             => None,
            };

            let update = match args.update {
                Present(ref u) if u.len() > 0 => Some(u),
                _                             => None,
            };

            let destroy = match args.destroy {
                Present(ref d) if d.len() > 0 => Some(d),
                _                             => None,
            };

            if let (None,None,None) = (create,update,destroy) {
                let mut rargs = SetResponseArgs::default();
                rargs.old_state = Some(old_state.clone());
                rargs.new_state = old_state;
                return Ok(rargs);
            }

            let new_state = try!(self.db.next_state(self.userid));

            let (created, not_created) = match create {
                None    => (BTreeMap::new(), BTreeMap::new()),
                Some(c) => try!(self.db.create_records(self.userid, c)),
            };

            let (updated, not_updated) = match update {
                None    => (Vec::new(), BTreeMap::new()),
                Some(u) => try!(self.db.update_records(self.userid, u)),
            };

            let (destroyed, not_destroyed) = match destroy {
                None    => (Vec::new(), BTreeMap::new()),
                Some(d) => try!(self.db.destroy_records(self.userid, d)),
            };

            Ok(SetResponseArgs {
                old_state:     Some(old_state),
                new_state:     new_state,
                created:       created,
                updated:       updated,
                destroyed:     destroyed,
                not_created:   not_created,
                not_updated:   not_updated,
                not_destroyed: not_destroyed,
            })
        }));

        Ok(res)
    }
}
