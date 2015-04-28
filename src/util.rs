use db::Db;

#[derive(Debug)]
pub struct RequestContext {
    pub userid: i64, // XXX would prefer u64 but sqlite integer type
    pub db: Db,
}
