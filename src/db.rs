use rusqlite::{SqliteConnection, SqliteError};
use rusqlite::types::{ToSql, FromSql};
use rustc_serialize::json::{Json, ParserError};
use jmap::util::{FromJson, ParseError};
use jmap::util::Presence::Present;
use jmap::method::{MethodError, ErrorDescription};
use jmap::contact::Contact;
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
    InternalError(String),
}

impl Error for DbError {
    fn description(&self) -> &str {
        match *self {
            StateTooOld      => "state too old",
            InternalError(_) => "internal database error",
        }
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            StateTooOld          => "state too old".to_string(),
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

    pub fn transaction<F,T>(&self, f: F) -> Result<T,DbError> where F: Fn() -> Result<T,DbError> {
        let nested;

        match self.in_txn.get() {
            false => {
                try!(self.exec("BEGIN DEFERRED", &[]));
                self.in_txn.set(true);
                nested = false;
            },
            true  => {
                try!(self.exec("SAVEPOINT sp", &[]));
                nested = true;
            },
        };

        let r = f();

        match r {
            Ok(_) => match nested {
                false => {
                    try!(self.exec("COMMIT", &[]));
                    self.in_txn.set(false);
                },
                true => try!(self.exec("RELEASE sp", &[])),
            },
            Err(_) => match nested {
                false => {
                    try!(self.exec("ROLLBACK", &[]));
                    self.in_txn.set(false);
                },
                true => try!(self.exec("ROLLBACK TO sp", &[])),
            },
        };

        r
    }

    fn exec(&self, sql: &str, params: &[&ToSql]) -> Result<(),DbError> {
        let mut stmt = try!(self.conn.prepare(sql));
        try!(stmt.execute(params));
        Ok(())
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

            println!("upgraded db to version {}", VERSION);

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

    pub fn get_records(&self, userid: i64, ids: Option<&Vec<String>>, since_state: Option<&String>) -> Result<Vec<Contact>,DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        self.transaction(|| {
            let modseq: i64;

            let mut sql = "SELECT json FROM records WHERE userid = ? AND type = ?".to_string();
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
}
