// NOTES
//   - Make sure to use the `now_utc_fmt` for Rfc3339 for time, otherwise, rusqlite format it wrong.

use super::DbBmc;
use modql::field::{SqliteField, SqliteFields};
use uuid::Uuid;
use zc_common::time::now_micro;

/// This method must be called when a model controller intends to create its entity with a new uid
pub fn prep_fields_for_create<MC>(fields: &mut SqliteFields)
where
	MC: DbBmc,
{
	fields.push(SqliteField::new("id", Uuid::now_v7()));
	prep_fields_for_create_uid_included(fields);
}

/// This assume the uid is included (won't be added)
pub fn prep_fields_for_create_uid_included(fields: &mut SqliteFields) {
	add_timestamps_for_create(fields);
}

/// This method must be calledwhen a Model Controller plans to update its entity.
pub fn prep_fields_for_update<MC>(fields: &mut SqliteFields)
where
	MC: DbBmc,
{
	add_timestamps_for_update(fields);
}

/// Update the timestamps info for create
/// (e.g., cid, ctime, and mid, mtime will be updated with the same values)
fn add_timestamps_for_create(fields: &mut SqliteFields) {
	let now = now_micro();
	fields.push(SqliteField::new("ctime", now));
	fields.push(SqliteField::new("mtime", now));
}

/// Update the timestamps info only for update.
/// (.e.g., only mid, mtime will be udpated)
fn add_timestamps_for_update(fields: &mut SqliteFields) {
	let now = now_micro();
	fields.push(SqliteField::new("mtime", now));
}
