use jmap::method::*;
use jmap::mailbox::Mailbox;

use util::RequestContext;
use record::RecordHandler;

pub trait MailboxHandler {
    fn get_mailboxes(&self, args: &GetRequestArgs<Mailbox>)              -> Result<GetResponseArgs<Mailbox>,MethodError>;
    fn get_mailbox_updates(&self, args: &GetUpdatesRequestArgs<Mailbox>) -> Result<(GetUpdatesResponseArgs<Mailbox>,Option<GetResponseArgs<Mailbox>>),MethodError>;
    fn set_mailboxes(&self, args: &SetRequestArgs<Mailbox>)              -> Result<SetResponseArgs<Mailbox>,MethodError>;
}

impl MailboxHandler for RequestContext {
    fn get_mailboxes(&self, args: &GetRequestArgs<Mailbox>) -> Result<GetResponseArgs<Mailbox>,MethodError> {
        self.get_records(args)
    }

    fn get_mailbox_updates(&self, args: &GetUpdatesRequestArgs<Mailbox>) -> Result<(GetUpdatesResponseArgs<Mailbox>,Option<GetResponseArgs<Mailbox>>),MethodError> {
        self.get_record_updates(args)
    }

    fn set_mailboxes(&self, args: &SetRequestArgs<Mailbox>) -> Result<SetResponseArgs<Mailbox>,MethodError> {
        self.set_records(args)
    }
}
