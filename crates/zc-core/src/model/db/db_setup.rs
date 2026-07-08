use crate::model::Result;
use rusqlite::Connection;

// Some notes:
// - Currently, the database is in-memory only, but it may become persistent at the session level.
// - All tables have an `id` used for same-db joins, and a `uid` which is a UUID blob,
//   intended for sharing outside of Rust or across databases.
// - `id` uses `AUTOINCREMENT` to ensure IDs are not reused if a row is deleted.
// - `MAIN_TABLES` are the main database tables for all metadata. They are designed to be relatively small and to scale well.
// - `CONTENT_TABLES` are designed to hold larger content and may have different trimming/cleaning strategies.
//    - A future strategy could involve having a set of content tables per run, using the b58 run.uid suffix. This would make it very fast to clean up old ones.
// - References between these two sets of tables are by `uid`, as they may eventually reside in different databases.

pub fn recreate_db(con: &Connection) -> Result<()> {
	create_schema(con)?;
	Ok(())
}

// region:    --- Main Tables

const RUN_TABLE: (&str, &str) = (
	"run",
	"
CREATE TABLE IF NOT EXISTS run (
		id          BLOB PRIMARY KEY,

		prompt      TEXT,
		answer      TEXT,
		error       TEXT,

		ctime  INTEGER NOT NULL,
		mtime  INTEGER NOT NULL,

		model       TEXT,
		aixc_idx_seq INTEGER NOT NULL DEFAULT 0
) STRICT, WITHOUT ROWID",
);

const AIXC_TABLE: (&str, &str) = (
	"aixc",
	"
CREATE TABLE IF NOT EXISTS aixc (
		id          BLOB PRIMARY KEY,
		run_id      BLOB NOT NULL,
		idx         INTEGER NOT NULL,
		label       TEXT,
		ctime       INTEGER NOT NULL,
		mtime       INTEGER NOT NULL,

		start       INTEGER,
		ai_start    INTEGER,
		ai_end      INTEGER,
		end         INTEGER,

		-- Model
		model_ov    TEXT,
		model_upstream TEXT,

		-- Prompt / Answer
		prompt_json TEXT,

		-- Response States
		answer_json TEXT,
		error       TEXT,
		end_state   TEXT,

		-- Usage
		usage_json  TEXT,
		token_in    INTEGER,
		token_out   INTEGER,
		token_reason INTEGER,
		token_cache_hit INTEGER,
		token_cache_write INTEGER,
		cost        REAL
) STRICT, WITHOUT ROWID",
);

const ALL_MAIN_TABLES: &[(&str, &str)] = &[RUN_TABLE, AIXC_TABLE];

// endregion: --- Main Tables

// region:    --- Support

fn create_schema(con: &Connection) -> Result<()> {
	for tables in [ALL_MAIN_TABLES] {
		for (_name, table_sql) in tables {
			con.execute(table_sql, ())?;
		}
	}

	Ok(())
}

// endregion: --- Support
