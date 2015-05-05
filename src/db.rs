use rusqlite::{SqliteConnection, SqliteTransaction, SqliteError};
use rusqlite::types::ToSql;
use rustc_serialize::json::{Json, ParserError};
use jmap::util::{FromJson, ParseError};
use jmap::contact::Contact;
use std::error::Error;
use std::convert::From;
use std::fmt;
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
    InternalError(String),
}

impl Error for DbError {
    fn description(&self) -> &str {
        match *self {
            InternalError(_) => "internal database error",
        }
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
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

pub type Transaction<'a> = SqliteTransaction<'a>;

#[derive(Debug)]
pub struct Db {
    conn: SqliteConnection,
}

impl Db {
    pub fn open() -> Result<Db,DbError> {
        //let conn = try!(SqliteConnection::open_in_memory());
        let conn = try!(SqliteConnection::open(&Path::new("db.sqlite")));
        let db = Db {
            conn: conn,
        };

        try!(db.upgrade());

        Ok(db)
    }

    fn transaction(&self) -> Result<Transaction,DbError> {
        match self.conn.transaction() {
            Ok(t) => Ok(t),
            Err(e) => Err(DbError::from(e)),
        }
    }

    fn exec(&self, sql: &str) -> Result<bool,DbError> {
        let mut stmt = try!(self.conn.prepare(sql));
        try!(stmt.execute(&[]));
        Ok(true)
    }

    fn version(&self) -> Result<u32,DbError> {
        let mut stmt = try!(self.conn.prepare("PRAGMA user_version"));
        let mut res = try!(stmt.query(&[]));
        let next = try!(res.next().unwrap());
        let v: i32 = next.get(0);
        Ok(v as u32)
    }

    fn set_version(&self, v: u32) -> Result<bool,DbError> {
        self.exec(format!("PRAGMA user_version = {}", v as i32).as_ref())
    }

    fn upgrade(&self) -> Result<bool,DbError> {
        let txn = try!(self.transaction());

        let ver = try!(self.version());
        if ver == VERSION { return Ok(true) }

        // new database
        if ver == 0 {
            for sql in CREATE_SQL.iter() {
                try!(self.exec(sql));
            }
        }

        /*
        // existing database, upgrade required
        else {
            // XXX 
        }
        */

        try!(self.set_version(VERSION));

        try!(txn.commit());

        println!("upgraded db to version {}", VERSION);

        Ok(true)
    }

    pub fn get_records(&self, userid: i64, ids: Option<&Vec<String>>) -> Result<Vec<Contact>,DbError> {
        let objtype = 1; // XXX contacts are type 1 for now

        let mut sql = "SELECT json FROM records WHERE userid = ? AND type = ?".to_string();
        let mut params: Vec<&ToSql> = vec!(&userid, &objtype);


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
    }
}
