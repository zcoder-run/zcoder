use crate::model::Result;
use crate::model::db::db_setup::recreate_db;
use modql::SqliteFromRow;
use rusqlite::types::FromSql;
use rusqlite::{Connection, OptionalExtension, Params};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Db {
	con: Arc<Mutex<Connection>>,
}

pub struct DbTx<'a> {
	tx: &'a rusqlite::Transaction<'a>,
}

/// Constructor & Setup
impl Db {
	pub fn new() -> Result<Self> {
		// let con = Connection::open(".mock-db.sqlite")?;
		let con = Connection::open_in_memory()?;
		let con = Arc::new(Mutex::new(con));

		Ok(Self { con })
	}

	pub fn recreate(&self) -> Result<()> {
		let con = self.con.lock()?;
		recreate_db(&con)?;
		Ok(())
	}

	pub fn exec_in_tx<R, F>(&self, f: F) -> Result<R>
	where
		F: FnOnce(&DbTx) -> Result<R>,
	{
		let mut conn_g = self.con.lock()?;
		let tx = conn_g.transaction()?;
		let tx_db = DbTx { tx: &tx };

		// exec the function
		let res = f(&tx_db);

		if res.is_ok() {
			tx.commit()?;
		} else {
			tx.rollback()?;
		}

		res
	}
}

// Executors
impl Db {
	/// Execute a parameterized sql with its params, and return the number of rows affected
	/// returns: number of rows affected
	pub fn exec(&self, sql: &str, params: impl Params) -> Result<usize> {
		let conn_g = self.con.lock()?;
		_exec(&conn_g, sql, params)
	}

	/// Perform a sql exec and return the first row and first value as num
	/// NOTE: This is useful for query with RETURNING ID
	/// e.g., `db.exec_as_num("select count(*) from person", [] )`
	pub fn exec_returning_num(&self, sql: &str, params: impl Params) -> Result<i64> {
		let conn_g = self.con.lock()?;
		_exec_returning_num(&conn_g, sql, params)
	}

	/// Perform a sql exec and returns the first value of the first row and
	/// cast it to the type T
	/// ```
	/// # use zc_core::Db;
	/// let db = Db::new().unwrap();
	/// db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)", []).unwrap();
	/// db.exec("INSERT INTO t (id, name) VALUES (42, 'foo')", []).unwrap();
	/// let id: i64 = db.exec_returning_as("SELECT id FROM t WHERE name = ?1", ["foo"]).unwrap();
	/// assert_eq!(id, 42);
	/// ```
	pub fn exec_returning_as<T: FromSql>(&self, sql: &str, params: impl Params) -> Result<T> {
		let conn_g = self.con.lock()?;
		_exec_returning_as(&conn_g, sql, params)
	}

	pub fn exec_returning_as_optional<T: FromSql>(&self, sql: &str, params: impl Params) -> Result<Option<T>> {
		let conn_g = self.con.lock()?;
		_exec_returning_as_optional(&conn_g, sql, params)
	}

	/// Fetch the first row and cast to to Option<T>
	/// NOTE: This assume the sql would have the LIMIT 1 added
	/// TODO: Might want to add the LIMIT 1 if not already (not sure)
	pub fn fetch_first<P, T>(&self, sql: &str, params: P) -> Result<Option<T>>
	where
		P: Params,
		T: SqliteFromRow,
	{
		let conn_g = self.con.lock()?;
		_fetch_first::<P, T>(&conn_g, sql, params)
	}

	pub fn fetch_all<P, T>(&self, sql: &str, params: P) -> Result<Vec<T>>
	where
		P: Params,
		T: SqliteFromRow,
	{
		let conn_g = self.con.lock()?;
		_fetch_all::<P, T>(&conn_g, sql, params)
	}
}

impl<'a> DbTx<'a> {
	pub fn exec(&self, sql: &str, params: impl Params) -> Result<usize> {
		_exec(self.tx, sql, params)
	}

	pub fn exec_returning_num(&self, sql: &str, params: impl Params) -> Result<i64> {
		_exec_returning_num(self.tx, sql, params)
	}

	pub fn exec_returning_as<T: FromSql>(&self, sql: &str, params: impl Params) -> Result<T> {
		_exec_returning_as(self.tx, sql, params)
	}

	pub fn exec_returning_as_optional<T: FromSql>(&self, sql: &str, params: impl Params) -> Result<Option<T>> {
		_exec_returning_as_optional(self.tx, sql, params)
	}

	pub fn fetch_first<P, T>(&self, sql: &str, params: P) -> Result<Option<T>>
	where
		P: Params,
		T: SqliteFromRow,
	{
		_fetch_first::<P, T>(self.tx, sql, params)
	}

	pub fn fetch_all<P, T>(&self, sql: &str, params: P) -> Result<Vec<T>>
	where
		P: Params,
		T: SqliteFromRow,
	{
		_fetch_all::<P, T>(self.tx, sql, params)
	}
}

// region:    --- Support

fn _exec(conn: &Connection, sql: &str, params: impl Params) -> Result<usize> {
	let row_affected = conn.execute(sql, params)?;
	Ok(row_affected)
}

fn _exec_returning_num(conn: &Connection, sql: &str, params: impl Params) -> Result<i64> {
	let mut stmt = conn.prepare(sql)?;
	let id = stmt.query_row(params, |r| r.get::<_, i64>(0))?;
	Ok(id)
}

fn _exec_returning_as<T: FromSql>(conn: &Connection, sql: &str, params: impl Params) -> Result<T> {
	let mut stmt = conn.prepare(sql)?;
	let res = stmt.query_row(params, |r| r.get::<_, T>(0))?;
	Ok(res)
}

fn _exec_returning_as_optional<T: FromSql>(conn: &Connection, sql: &str, params: impl Params) -> Result<Option<T>> {
	let mut stmt = conn.prepare(sql)?;
	let res = stmt.query_row(params, |r| r.get::<_, T>(0)).optional()?;
	Ok(res)
}

fn _fetch_first<P, T>(conn: &Connection, sql: &str, params: P) -> Result<Option<T>>
where
	P: Params,
	T: SqliteFromRow,
{
	let all: Vec<T> = _fetch_all(conn, sql, params)?;
	Ok(all.into_iter().next())
}

fn _fetch_all<P, T>(conn: &Connection, sql: &str, params: P) -> Result<Vec<T>>
where
	P: Params,
	T: SqliteFromRow,
{
	let mut stmt = conn.prepare(sql)?;
	let iter = stmt.query_and_then(params, |r| T::sqlite_from_row(r))?;
	let mut res = Vec::new();
	for item in iter {
		res.push(item?)
	}
	Ok(res)
}

// endregion: --- Support
