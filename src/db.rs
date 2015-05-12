use rusqlite::{SqliteConnection, SqliteError};
use rusqlite::types::{ToSql, FromSql};
use rustc_serialize::json::{Json, ToJson, ParserError};
use uuid::Uuid;
use jmap::parse::{FromJson, ParseError};
use jmap::parse::Presence::Present;
use jmap::method::{MethodError, ErrorDescription, SetError};
use jmap::contact::{Contact, PartialContact};
use std::collections::BTreeMap;
use std::error::Error;
use std::convert::From;
use std::cell::Cell;
use std::fmt;
use std::cmp;
use self::DbError::*;

use std::path::Path;

const VERSION: u32 = 1;

const CREATE_SQL: [&'static str; 6] = [
r###"
CREATE TABLE records (
    rowid       INTEGER PRIMARY KEY,
    id          TEXT NOT NULL,
    userid      INTEGER NOT NULL,
    type        INTEGER NOT NULL,
    modseq      INTEGER NOT NULL,
    deleted     INTEGER NOT NULL DEFAULT 0,
    json        TEXT NOT NULL,
    UNIQUE( id, userid )
);
"###,
r###"
CREATE INDEX idx_record_id_userid          ON records ( id, userid );
"###,
r###"
CREATE INDEX idx_record_userid_type        ON records ( userid, type );
"###,
r###"
CREATE INDEX idx_record_userid_type_modseq ON records ( userid, type, modseq );
"###,
r###"
CREATE TABLE modseq (
    userid      INTEGER NOT NULL,
    type        INTEGER NOT NULL,
    modseq      INTEGER NOT NULL,
    low_modseq  INTEGER NOT NULL,
    UNIQUE( userid, type )
);
"###,
r###"
CREATE INDEX idx_userid_type ON modseq ( userid, type );
"###,
];

/*
const UPGRADE_SQL: [&'static str; 1] = [
    // v1
    ""
];
*/

#[derive(Clone, PartialEq, Debug)]
pub enum DbError {
    StateTooOld,
    StateMismatch,
    TooManyChanges,
    InternalError(String),
}

impl Error for DbError {
    fn description(&self) -> &str {
        match *self {
            StateTooOld      => "state too old",
            StateMismatch    => "state mismatch",
            TooManyChanges   => "too many changes",
            InternalError(_) => "internal database error",
        }
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            StateTooOld          => "state too old".to_string(),
            StateMismatch        => "state mismatch".to_string(),
            TooManyChanges       => "too many changes".to_string(),
            InternalError(ref e) => format!("internal database error: {}", e),
        }.to_string())
    }
}

impl From<SqliteError> for DbError {
    fn from(e: SqliteError) -> DbError {
        InternalError(format!("sqlite: {}", e))
    }
}

impl From<ParserError> for DbError {
    fn from(e: ParserError) -> DbError {
        InternalError(format!("json: {}", e))
    }
}

impl From<ParseError> for DbError {
    fn from(e: ParseError) -> DbError {
        InternalError(format!("jmap: {}", e))
    }
}

impl From<DbError> for MethodError {
    fn from(e: DbError) -> MethodError {
        match e {
            StateTooOld      => MethodError::CannotCalculateChanges,
            StateMismatch    => MethodError::StateMismatch,
            TooManyChanges   => MethodError::TooManyChanges,
            InternalError(_) => MethodError::InternalError(Present(ErrorDescription(format!("{}", e)))),
        }
    }
}

#[derive(Debug)]
pub struct Db {
    conn:   SqliteConnection,
    in_txn: Cell<bool>,
}

impl Db {
    pub fn open() -> Result<Db,DbError> {
        //let conn = try!(SqliteConnection::open_in_memory());
        let conn = try!(SqliteConnection::open(&Path::new("db.sqlite")));
        let db = Db {
            conn:   conn,
            in_txn: Cell::new(false),
        };

        try!(db.upgrade());

        Ok(db)
    }

    fn do_transaction<F,T>(&self, f: F, nested: bool) -> Result<T,DbError> where F: Fn() -> Result<T,DbError> {
        let r = f();

        match r {
            Ok(_) => match nested {
                false => {
                    try!(self.exec("COMMIT", &[]));
                    self.in_txn.set(false);
                },
                true => {
                    try!(self.exec("RELEASE sp", &[]));
                },
            },
            Err(_) => match nested {
                false => {
                    try!(self.exec("ROLLBACK", &[]));
                    self.in_txn.set(false);
                },
                true => {
                    try!(self.exec("ROLLBACK TO sp", &[]));
                },
            },
        };

        r
    }

    pub fn exclusive<F,T>(&self, f: F) -> Result<T,DbError> where F: Fn() -> Result<T,DbError> {
        try!(self.exec("BEGIN EXCLUSIVE", &[]));
        self.in_txn.set(true);
        self.do_transaction(f, false)
    }

    pub fn transaction<F,T>(&self, f: F) -> Result<T,DbError> where F: Fn() -> Result<T,DbError> {
        let nested = match self.in_txn.get() {
            false => {
                try!(self.exec("BEGIN DEFERRED", &[]));
                self.in_txn.set(true);
                false
            },
            true  => {
                try!(self.exec("SAVEPOINT sp", &[]));
                true
            },
        };
        self.do_transaction(f, nested)
    }

    fn exec(&self, sql: &str, params: &[&ToSql]) -> Result<usize,DbError> {
        let mut stmt = try!(self.conn.prepare(sql));
        Ok(try!(stmt.execute(params)) as usize)
    }

    fn exec_value<T>(&self, sql: &str, params: &[&ToSql]) -> Result<Option<T>,DbError> where T: FromSql {
        let mut stmt = try!(self.conn.prepare(sql));
        let mut res = try!(stmt.query(params));

        match res.next() {
            None       => Ok(None),
            Some(next) =>
                match next {
                    Err(e)   => Err(InternalError(format!("sqlite: {}", e))),
                    Ok(next) => {
                        let v: T = next.get(0);
                        Ok(Some(v))
                    },
            }
        }
    }

    fn version(&self) -> Result<u32,DbError> {
        let v = try!(self.exec_value::<i32>("PRAGMA user_version", &[]));
        match v {
            Some(v) => Ok(v as u32),
            None    => Ok(0),
        }
    }

    fn set_version(&self, v: u32) -> Result<(),DbError> {
        try!(self.exec(format!("PRAGMA user_version = {}", v as i32).as_ref(), &[]));
        Ok(())
    }

    fn upgrade(&self) -> Result<(),DbError> {
        self.transaction(|| {
            let ver = try!(self.version());
            if ver == VERSION { return Ok(()) }

            // new database
            if ver == 0 {
                for sql in CREATE_SQL.iter() {
                    try!(self.exec(sql, &[]));
                }
            }

            /*
            // existing database, upgrade required
            else {
                // XXX 
            }
            */

            try!(self.set_version(VERSION));

            info!("upgraded db to version {}", VERSION);

            Ok(())
        })
    }

    pub fn get_state(&self, userid: i64) -> Result<String,DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        let params: Vec<&ToSql> = vec!(&userid, &objtype);
        let sv = try!(self.exec_value::<i64>("SELECT modseq FROM modseq WHERE userid = ? AND type = ?", params.as_ref()));
        match sv {
            None    => Ok("0".to_string()),
            Some(v) => Ok(v.to_string()),
        }
    }

    pub fn check_state(&self, userid: i64, state: &String) -> Result<(),DbError> {
        let s = try!(self.get_state(userid));
        match s == *state {
            true  => Ok(()),
            false => Err(StateMismatch),
        }
    }

    pub fn next_state(&self, userid: i64) -> Result<String,DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        let params: Vec<&ToSql> = vec!(&userid, &objtype);

        self.transaction(|| {
            if let 0 = try!(self.exec("UPDATE modseq SET modseq = (modseq+1) WHERE userid = ? AND type = ?", &params)) {
                try!(self.exec("INSERT INTO modseq ( userid, type, modseq, low_modseq ) VALUES ( ?, ?, 1, 1 )", &params));
            }
            self.get_state(userid)
        })
    }

    pub fn get_records(&self, userid: i64, ids: Option<&Vec<String>>, since_state: Option<&String>) -> Result<Vec<Contact>,DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        self.transaction(|| {
            let modseq: i64;

            let mut sql = "SELECT json FROM records WHERE userid = ? AND type = ? AND deleted = 0".to_string();
            let mut params: Vec<&ToSql> = vec!(&userid, &objtype);

            if let Some(ref since_state) = since_state {
                let parsed = since_state.parse::<i64>();
                modseq = match parsed {
                    Err(_) => 0,
                    Ok(i)  => cmp::max(i,0),
                };

                let mv = try!(self.exec_value::<i64>("SELECT low_modseq FROM modseq WHERE userid = ? AND type = ?", params.as_ref()));
                let valid = match mv {
                    None    => false,
                    Some(v) => v <= modseq,
                };
                if let false = valid {
                    return Err(StateTooOld);
                }

                sql.push_str(" AND modseq > ?");
                params.push(&modseq);
            }

            if let Some(ref ids) = ids {
                sql.push_str(" AND id IN ( ");

                let mut i = ids.iter();
                if let Some(id) = i.next() {
                    sql.push_str("?");
                    params.push(id);
                }
                for id in i {
                    sql.push_str(",?");
                    params.push(id);
                }

                sql.push_str(" )");
            }

            let mut stmt = try!(self.conn.prepare(sql.as_ref()));
            let res = try!(stmt.query(params.as_ref()));

            let mut records: Vec<Contact> = Vec::new();
            for row in res {
                if let Ok(ref r) = row {
                    let json = try!(Json::from_str((r.get::<String>(0)).as_ref()));
                    records.push(try!(Contact::from_json(&json)));
                }
            }

            Ok(records)
        })
    }

    pub fn get_record_updates(&self, userid: i64, since_state: &String, max_changes: Option<i64>) -> Result<(Vec<String>,Vec<String>),DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        self.transaction(|| {
            let parsed = since_state.parse::<i64>();
            let modseq = match parsed {
                Err(_) => 0,
                Ok(i)  => cmp::max(i,0),
            };

            let max;
            let max1;

            let mut params: Vec<&ToSql> = vec!(&userid, &objtype);

            let mv = try!(self.exec_value::<i64>("SELECT low_modseq FROM modseq WHERE userid = ? AND type = ?", params.as_ref()));
            let valid = match mv {
                None    => false,
                Some(v) => v <= modseq,
            };
            if let false = valid {
                return Err(StateTooOld);
            }

            params.push(&modseq);

            let mut sql = " FROM records WHERE userid = ? AND type = ? AND modseq > ?".to_string();

            if let Some(max_changes) = max_changes {
                sql.push_str(" LIMIT ?");

                max1 = max_changes + 1;
                params.push(&max1);

                let mut s = "SELECT COUNT(*)".to_string();
                s.push_str(sql.as_ref());

                let count = try!(self.exec_value::<i64>(s.as_ref(), params.as_ref())).unwrap();
                if count >= max1 {
                    return Err(TooManyChanges);
                }

                params.pop();
                max = max_changes;
                params.push(&max);
            }

            let mut s = "SELECT id,deleted".to_string();
            s.push_str(sql.as_ref());

            let mut stmt = try!(self.conn.prepare(s.as_ref()));
            let res = try!(stmt.query(params.as_ref()));

            let mut changed: Vec<String> = Vec::new();
            let mut removed: Vec<String> = Vec::new();
            for row in res {
                if let Ok(ref r) = row {
                    let id = r.get::<String>(0);
                    let deleted = r.get::<i64>(1) == 1;
                    match deleted {
                        true  => removed.push(id),
                        false => changed.push(id),
                    }
                }
            }

            Ok((changed, removed))
        })
    }

    pub fn create_records(&self, userid: i64, create: &BTreeMap<String,PartialContact>) -> Result<(BTreeMap<String,PartialContact>,BTreeMap<String,SetError>),DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        // XXX spec doesn't list any reasons why a create could fail (SetError)
        // so for now we'll always return an empty notCreated list
        self.transaction(|| {
            let mut stmt = try!(self.conn.prepare("INSERT INTO records ( userid, type, modseq, id, json ) VALUES ( ?, ?, (SELECT modseq FROM modseq WHERE userid = ? AND type = ?), ?, ?)"));
            let params: Vec<&ToSql> = vec!(&userid, &objtype, &userid, &objtype);

            // iterative style so we can use try!
            let mut created = BTreeMap::new();
            for (client_id, pc) in create.iter() {
                let id = Uuid::new_v4().to_hyphenated_string();
                let mut c = Contact::default().updated_with(&pc);
                // XXX invalidArguments if incoming has an id already
                c.id = id.clone();
                let json = c.to_json().to_string();
                let mut p = params.clone();
                p.push(&c.id);
                p.push(&json);
                try!(stmt.execute(&p));
                let mut cpc = PartialContact::default();
                cpc.id = Present(id);
                created.insert(client_id.clone(), cpc);
            }
            let not_created: BTreeMap<String,SetError> = BTreeMap::new();
            Ok((created, not_created))
        })
    }

    pub fn update_records(&self, userid: i64, update: &BTreeMap<String,PartialContact>) -> Result<(Vec<String>,BTreeMap<String,SetError>),DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        // XXX spec doesn't list any reasons why a update could fail (SetError)
        // so for now we'll always return an empty notUpdated list
        self.transaction(|| {
            let params: Vec<&ToSql> = vec!(&userid, &objtype);

            let mut update_stmt = try!(self.conn.prepare("UPDATE records SET modseq = (SELECT modseq FROM modseq WHERE userid = ? AND type = ?), json = ? WHERE userid = ? AND type = ? AND id = ?"));

            // iterative style so we can use try!
            let mut updated = Vec::new();
            for (id, pc) in update.iter() {
                let mut get_p = params.clone();
                get_p.push(id);
                let json = try!(self.exec_value::<String>("SELECT json FROM records WHERE userid = ? AND type = ? AND deleted = 0 AND id = ?", &get_p)); // XXX stmt version of exec_value?

                // XXX None means not found, must return some sane error

                if let Some(j) = json {
                    // XXX assuming parse success
                    let c = Contact::from_json(&Json::from_str(j.as_ref()).unwrap()).unwrap().updated_with(&pc);
                    // XXX invalidArguments if trying to change id (or other immutable params?)
                    let new_json = c.to_json().to_string();
                    let mut p = params.clone();
                    p.push(&new_json);
                    p.push(&userid); // XXX merp, I need a better way to build these
                    p.push(&objtype);
                    p.push(&c.id);
                    try!(update_stmt.execute(&p));
                    updated.push(id.clone());
                }
            }
            let not_updated: BTreeMap<String,SetError> = BTreeMap::new();
            Ok((updated, not_updated))
        })
    }

    pub fn destroy_records(&self, userid: i64, destroy: &Vec<String>) -> Result<(Vec<String>,BTreeMap<String,SetError>),DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        // XXX spec doesn't list any reasons why a destroy could fail (SetError)
        // so for now we'll always return an empty notDestroyed list
        self.transaction(|| {
            let mut stmt = try!(self.conn.prepare("UPDATE records SET deleted = 1, modseq = (SELECT modseq FROM modseq WHERE userid = ? AND type = ?) WHERE userid = ? AND type = ? AND id = ? AND deleted = 0"));
            let params: Vec<&ToSql> = vec!(&userid, &objtype, &userid, &objtype);

            // iterative style so we can use try!
            let mut destroyed = Vec::new();
            for id in destroy.iter() {
                let mut p = params.clone();
                p.push(id);
                try!(stmt.execute(&p));
                destroyed.push(id.clone());
            }
            let not_destroyed: BTreeMap<String,SetError> = BTreeMap::new();
            Ok((destroyed, not_destroyed))
        })
    }
}
