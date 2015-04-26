use rusqlite::{SqliteConnection, SqliteTransaction, SqliteError};

const VERSION: u32 = 1;

const CREATE_SQL: &'static str = r"
CREATE TABLE objects (
    rowid       INTEGER PRIMARY KEY,
    id          TEXT NOT NULL,
    json        TEXT NOT NULL,
    UNIQUE( id )
);
CREATE INDEX idx_object_id ON objects ( id );
";

/*
const UPGRADE_SQL: [&'static str; 1] = [
    // v1
    ""
];
*/

pub type Transaction<'a> = SqliteTransaction<'a>;
pub type Error = SqliteError;

#[derive(Debug)]
pub struct Db {
    conn: SqliteConnection,
}

impl Db {
    pub fn open() -> Result<Db,Error> {
        let conn = try!(SqliteConnection::open_in_memory());
        let db = Db {
            conn: conn,
        };

        try!(db.upgrade());

        Ok(db)
    }

    fn transaction(&self) -> Result<Transaction,Error> {
        self.conn.transaction()
    }

    fn exec(&self, sql: &str) -> Result<bool,Error> {
        let mut stmt = try!(self.conn.prepare(sql));
        try!(stmt.execute(&[]));
        Ok(true)
    }

    fn version(&self) -> Result<u32,Error> {
        let mut stmt = try!(self.conn.prepare("PRAGMA user_version"));
        let mut res = try!(stmt.query(&[]));
        let next = try!(res.next().unwrap());
        let v: i32 = next.get(0);
        Ok(v as u32)
    }

    fn set_version(&self, v: u32) -> Result<bool,Error> {
        self.exec(format!("PRAGMA user_version = {}", v as i32).as_ref())
    }

    fn upgrade(&self) -> Result<bool,Error> {
        let txn = try!(self.transaction());

        let ver = try!(self.version());
        if ver == VERSION { return Ok(true) }

        // new database
        if ver == 0 {
            try!(self.exec(CREATE_SQL));
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
}
