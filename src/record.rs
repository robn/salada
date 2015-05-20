use std::collections::{HashSet, BTreeMap};

use jmap::record::Record;
use jmap::method::*;
use jmap::parse::Presence::*;

use util::RequestContext;
use db::RecordType;

pub trait RecordHandler<R: Record> {
    fn get_records(&self, args: &GetRequestArgs<R>)               -> Result<GetResponseArgs<R>,MethodError>;
    fn get_record_updates(&self, args: &GetUpdatesRequestArgs<R>) -> Result<(GetUpdatesResponseArgs<R>,Option<GetResponseArgs<R>>),MethodError>;
    fn set_records(&self, args: &SetRequestArgs<R>)               -> Result<SetResponseArgs<R>,MethodError>;
}

impl<R: Record> RecordHandler<R> for RequestContext where R: RecordType {
    fn get_records(&self, args: &GetRequestArgs<R>) -> Result<GetResponseArgs<R>,MethodError> {
        let (records, state): (Vec<R>, String) = try!(self.db.transaction(|| {
            Ok((
                try!(self.db.get_records::<R>(self.userid, args.ids.as_option(), args.since_state.as_option())),
                try!(self.db.get_state::<R>(self.userid)),
            ))
        }));

        let not_found = match args.ids {
            Absent => None,
            Present(ref ids) => {
                let found: HashSet<String> = ids.iter().cloned().collect();
                let not_found = ids.into_iter().filter(|id| !found.contains(*id)).map(|id| id.clone()).collect::<Vec<_>>();
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
            ..Default::default()
        };

        Ok(response)
    }

    fn get_record_updates(&self, args: &GetUpdatesRequestArgs<R>) -> Result<(GetUpdatesResponseArgs<R>,Option<GetResponseArgs<R>>),MethodError> {
        let (changed, removed, state) = try!(self.db.transaction(|| {
            let max_changes = match args.max_changes.as_option() {
                Some(i) => Some(*i as i64),
                None    => None,
            };
            let (changed, removed) = try!(self.db.get_record_updates::<R>(self.userid, &args.since_state, max_changes));
            Ok((
                changed,
                removed,
                try!(self.db.get_state::<R>(self.userid)),
            ))
        }));

        let records_response = match args.fetch_records {
            Present(true) => {
                let get_records_args = GetRequestArgs::<R> {
                    ids:         Present(changed.clone()),
                    properties:  args.fetch_record_properties.clone(),
                    ..Default::default()
                };
                match self.get_records(&get_records_args) {
                    Ok(r) => Some(r),
                    _     => None, // XXX what should I do if getRecords fails?
                }
            },
            _ => None,
        };

        let response = GetUpdatesResponseArgs {
            old_state: args.since_state.clone(),
            new_state: state,
            changed:   changed,
            removed:   removed,
            ..Default::default()
        };

        Ok((response, records_response))
    }

    fn set_records(&self, args: &SetRequestArgs<R>) -> Result<SetResponseArgs<R>,MethodError> {
        let res = try!(self.db.exclusive(|| {
            if let Present(ref s) = args.if_in_state {
                try!(self.db.check_state::<R>(self.userid, s));
            }

            let old_state = try!(self.db.get_state::<R>(self.userid));

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

            let new_state = try!(self.db.next_state::<R>(self.userid));

            let (created, not_created) = match create {
                None    => (BTreeMap::new(), BTreeMap::new()),
                Some(c) => try!(self.db.create_records::<R>(self.userid, c)),
            };

            let (updated, not_updated) = match update {
                None    => (Vec::new(), BTreeMap::new()),
                Some(u) => try!(self.db.update_records::<R>(self.userid, u)),
            };

            let (destroyed, not_destroyed) = match destroy {
                None    => (Vec::new(), BTreeMap::new()),
                Some(d) => try!(self.db.destroy_records::<R>(self.userid, d)),
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
                ..Default::default()
            })
        }));

        Ok(res)
    }
}
