use jmap::method::*;
use jmap::calendar::Calendar;

use util::RequestContext;
use record::RecordHandler;

pub trait CalendarHandler {
    fn get_calendars(&self, args: &GetRequestArgs<Calendar>)               -> Result<GetResponseArgs<Calendar>,MethodError>;
    fn get_calendar_updates(&self, args: &GetUpdatesRequestArgs<Calendar>) -> Result<(GetUpdatesResponseArgs<Calendar>,Option<GetResponseArgs<Calendar>>),MethodError>;
    fn set_calendars(&self, args: &SetRequestArgs<Calendar>)               -> Result<SetResponseArgs<Calendar>,MethodError>;
}

impl CalendarHandler for RequestContext {
    fn get_calendars(&self, args: &GetRequestArgs<Calendar>) -> Result<GetResponseArgs<Calendar>,MethodError> {
        self.get_records(args)
    }

    fn get_calendar_updates(&self, args: &GetUpdatesRequestArgs<Calendar>) -> Result<(GetUpdatesResponseArgs<Calendar>,Option<GetResponseArgs<Calendar>>),MethodError> {
        self.get_record_updates(args)
    }

    fn set_calendars(&self, args: &SetRequestArgs<Calendar>) -> Result<SetResponseArgs<Calendar>,MethodError> {
        self.set_records(args)
    }
}
